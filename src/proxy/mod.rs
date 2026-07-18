pub mod adapter;
pub mod context_engine;
pub mod error;
pub mod fallback;
pub mod handler;
pub mod health;
pub mod metrics;
pub mod models;
pub mod translate;
pub mod util;

use std::sync::Arc;

use anyhow::Result;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use tokio::sync::RwLock;

use crate::config::ClaudexConfig;
use crate::context::rag::RagIndex;
use crate::context::sharing::SharedContext;
use metrics::MetricsStore;

pub struct ProxyState {
    pub config: Arc<RwLock<ClaudexConfig>>,
    pub metrics: MetricsStore,
    pub http_client: reqwest::Client,
    pub health_status: Arc<RwLock<health::HealthMap>>,
    pub circuit_breakers: fallback::CircuitBreakerMap,
    pub shared_context: SharedContext,
    pub rag_index: Option<RagIndex>,
    pub token_manager: crate::oauth::manager::TokenManager,
}

/// 获取 proxy 日志文件路径（~/.cache/claudex/proxy-{timestamp}-{pid}.log）
/// 每次启动生成独立日志文件，支持多实例并行
pub fn proxy_log_path() -> Option<std::path::PathBuf> {
    dirs::cache_dir().map(|d| {
        let ts = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let pid = std::process::id();
        d.join("claudex").join(format!("proxy-{ts}-{pid}.log"))
    })
}

pub async fn start_proxy(config: ClaudexConfig, port_override: Option<u16>) -> Result<()> {
    let port = port_override.unwrap_or(config.proxy_port);
    let host = config.proxy_host.clone();
    let max_body_bytes = config.proxy_max_body_mb.saturating_mul(1024 * 1024);

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    // Build RAG index if enabled
    let rag_index = if config.context.rag.enabled {
        let index = RagIndex::new(config.context.rag.clone());
        if let Some((base_url, api_key, _)) = crate::context::resolve_profile_endpoint(
            &config,
            &config.context.rag.profile,
            &config.context.rag.model,
        ) {
            if let Err(e) = index.build_index(&http_client, &base_url, &api_key).await {
                tracing::warn!("failed to build RAG index: {e}");
            }
        } else {
            tracing::warn!(
                profile = %config.context.rag.profile,
                "RAG profile not found, skipping index build"
            );
        }
        Some(index)
    } else {
        None
    };

    let token_manager = crate::oauth::manager::TokenManager::new(http_client.clone());

    let state = Arc::new(ProxyState {
        config: Arc::new(RwLock::new(config)),
        metrics: MetricsStore::new(),
        http_client,
        health_status: Arc::new(RwLock::new(health::HealthMap::new())),
        circuit_breakers: fallback::new_circuit_breaker_map(),
        shared_context: SharedContext::new(),
        rag_index,
        token_manager,
    });

    health::spawn_health_checker(state.clone());

    let app = build_router(state, max_body_bytes);

    let bind_addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    tracing::info!("proxy listening on {bind_addr}");

    crate::process::daemon::write_pid(std::process::id())?;

    axum::serve(listener, app).await?;

    crate::process::daemon::remove_pid()?;
    Ok(())
}

fn build_router(state: Arc<ProxyState>, max_body_bytes: usize) -> Router {
    Router::new()
        .route("/v1/models", get(models::list_models))
        .route(
            "/proxy/{profile}/v1/messages",
            post(handler::handle_messages),
        )
        .route("/health", get(|| async { "ok" }))
        .layer(DefaultBodyLimit::max(max_body_bytes))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn test_state() -> Arc<ProxyState> {
        let http_client = reqwest::Client::new();
        Arc::new(ProxyState {
            config: Arc::new(RwLock::new(ClaudexConfig::default())),
            metrics: MetricsStore::new(),
            http_client: http_client.clone(),
            health_status: Arc::new(RwLock::new(health::HealthMap::new())),
            circuit_breakers: fallback::new_circuit_breaker_map(),
            shared_context: SharedContext::new(),
            rag_index: None,
            token_manager: crate::oauth::manager::TokenManager::new(http_client),
        })
    }

    fn large_json_request(payload_len: usize) -> Request<Body> {
        let body = format!(r#"{{"model":"m","pad":"{}"}}"#, "a".repeat(payload_len));
        Request::builder()
            .method("POST")
            .uri("/proxy/nonexistent/v1/messages")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap()
    }

    #[tokio::test]
    async fn body_over_limit_is_rejected_with_413() {
        let app = build_router(test_state(), 1024 * 1024);
        let resp = app
            .oneshot(large_json_request(2 * 1024 * 1024))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn body_within_limit_reaches_handler() {
        // 3MB body exceeds axum's built-in 2MB default; with the configured
        // limit it must reach the handler (404: profile not found), not 413.
        let max_body = ClaudexConfig::default().proxy_max_body_mb * 1024 * 1024;
        let app = build_router(test_state(), max_body);
        let resp = app
            .oneshot(large_json_request(3 * 1024 * 1024))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
