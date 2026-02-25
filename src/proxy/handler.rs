use std::sync::Arc;
use std::time::Instant;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::Value;

use crate::config::{ProfileConfig, ProviderType};
use crate::proxy::ProxyState;
use crate::router::classifier;

pub async fn handle_messages(
    State(state): State<Arc<ProxyState>>,
    Path(profile_name): Path<String>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let start = Instant::now();

    let mut body_value: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")).into_response();
        }
    };

    // --- Smart Routing: resolve "auto" profile ---
    let resolved_profile_name = if profile_name == "auto" {
        resolve_auto_profile(&state, &body_value).await
    } else {
        profile_name.clone()
    };

    let config = state.config.read().await;

    let profile = match config.find_profile(&resolved_profile_name) {
        Some(p) => p.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                format!("profile '{resolved_profile_name}' not found"),
            )
                .into_response();
        }
    };

    if !profile.enabled {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            format!("profile '{resolved_profile_name}' is disabled"),
        )
            .into_response();
    }

    // Collect backup provider profiles
    let backup_profiles: Vec<ProfileConfig> = profile
        .backup_providers
        .iter()
        .filter_map(|name| config.find_profile(name).cloned())
        .filter(|p| p.enabled)
        .collect();

    let context_config = config.context.clone();
    let full_config = config.clone();
    let metrics = state.metrics.get_or_create(&resolved_profile_name);
    drop(config);

    // --- Context Engine: apply pre-processing ---
    super::middleware::apply_context_engine(
        &mut body_value,
        &state,
        &resolved_profile_name,
        &context_config,
        &full_config,
    )
    .await;

    let is_streaming = body_value
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // --- Circuit Breaker + Failover ---
    // Try primary provider
    let primary_result =
        try_with_circuit_breaker(&state, &profile, &headers, &body_value, is_streaming).await;

    let result = match primary_result {
        Ok(response) => Ok(response),
        Err(primary_err) => {
            tracing::warn!(
                profile = %profile.name,
                error = %primary_err,
                "primary provider failed, trying backups"
            );

            // Try backup providers in order
            let mut last_err = primary_err;
            let mut success = None;

            for backup in &backup_profiles {
                match try_with_circuit_breaker(&state, backup, &headers, &body_value, is_streaming)
                    .await
                {
                    Ok(response) => {
                        tracing::info!(
                            backup = %backup.name,
                            "failover succeeded"
                        );
                        success = Some(response);
                        break;
                    }
                    Err(e) => {
                        tracing::warn!(
                            backup = %backup.name,
                            error = %e,
                            "backup provider also failed"
                        );
                        last_err = e;
                    }
                }
            }

            match success {
                Some(response) => Ok(response),
                None => Err(last_err),
            }
        }
    };

    let latency = start.elapsed();

    match result {
        Ok(response) => {
            metrics.record_request(true, latency, 0);
            response
        }
        Err(e) => {
            metrics.record_request(false, latency, 0);
            tracing::error!(profile = %resolved_profile_name, error = %e, "proxy request failed");
            (StatusCode::BAD_GATEWAY, format!("proxy error: {e}")).into_response()
        }
    }
}

/// Resolve "auto" profile via smart router
async fn resolve_auto_profile(state: &ProxyState, body: &Value) -> String {
    let config = state.config.read().await;

    if !config.router.enabled {
        let default = config.router.resolve_profile("default").unwrap_or_else(|| {
            config
                .enabled_profiles()
                .first()
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "default".to_string())
        });
        return default;
    }

    let router_config = config.router.clone();

    // Resolve classifier profile endpoint
    let endpoint = crate::context::resolve_profile_endpoint(
        &config,
        &router_config.profile,
        &router_config.model,
    );
    drop(config);

    let user_message = classifier::extract_last_user_message(body).unwrap_or_default();

    if user_message.is_empty() {
        return router_config
            .resolve_profile("default")
            .unwrap_or_else(|| "default".to_string());
    }

    let (base_url, api_key, model) = match endpoint {
        Some(v) => v,
        None => {
            tracing::warn!(
                profile = %router_config.profile,
                "router classifier profile not found, using default"
            );
            return router_config
                .resolve_profile("default")
                .unwrap_or_else(|| "default".to_string());
        }
    };

    match classifier::classify_intent(&base_url, &api_key, &model, &user_message, &state.http_client).await {
        Ok(intent) => {
            let profile_name = router_config.resolve_profile(&intent).unwrap_or_else(|| {
                router_config
                    .resolve_profile("default")
                    .unwrap_or_else(|| "default".to_string())
            });
            tracing::info!(intent = %intent, profile = %profile_name, "smart routing resolved");
            profile_name
        }
        Err(e) => {
            tracing::warn!(error = %e, "intent classification failed, using default");
            router_config
                .resolve_profile("default")
                .unwrap_or_else(|| "default".to_string())
        }
    }
}

/// Try forwarding to a single provider with circuit breaker protection
async fn try_with_circuit_breaker(
    state: &ProxyState,
    profile: &ProfileConfig,
    headers: &HeaderMap,
    body: &Value,
    is_streaming: bool,
) -> anyhow::Result<Response> {
    // Check circuit breaker (single lock scope to avoid race condition)
    {
        let mut map = state.circuit_breakers.write().await;
        let cb = map
            .entry(profile.name.clone())
            .or_insert_with(Default::default);
        if !cb.can_attempt() {
            anyhow::bail!("circuit breaker open for profile '{}'", profile.name);
        }
    }
    // Lock is released here — forward can take seconds, don't hold it

    let result = try_forward(state, profile, headers, body, is_streaming).await;

    // Record result atomically
    let mut map = state.circuit_breakers.write().await;
    let cb = map
        .entry(profile.name.clone())
        .or_insert_with(Default::default);
    match &result {
        Ok(_) => cb.record_success(),
        Err(_) => cb.record_failure(),
    }
    drop(map);

    result
}

/// Forward request to a single provider (used for both primary and backup).
/// For non-streaming responses, also extracts and stores context for sharing.
async fn try_forward(
    state: &ProxyState,
    profile: &ProfileConfig,
    _headers: &HeaderMap,
    body: &Value,
    is_streaming: bool,
) -> anyhow::Result<Response> {
    match profile.provider_type {
        ProviderType::DirectAnthropic => forward_direct(state, profile, body, is_streaming).await,
        ProviderType::OpenAICompatible => {
            forward_translated(state, profile, body, is_streaming).await
        }
    }
}

/// Extract assistant text from an Anthropic-format response and store for sharing.
/// Only works for non-streaming responses where the body is available.
fn extract_and_store_context(state: &ProxyState, profile_name: &str, resp_body: &Value) {
    // Anthropic format: {"content": [{"type": "text", "text": "..."}]}
    let text = resp_body
        .get("content")
        .and_then(|c| c.as_array())
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|b| {
                    if b.get("type").and_then(|t| t.as_str()) == Some("text") {
                        b.get("text").and_then(|t| t.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    if text.len() >= 100 {
        let truncated = if text.len() > 500 {
            format!("{}...", &text[..500])
        } else {
            text
        };
        let shared_context = state.shared_context.clone();
        let name = profile_name.to_string();
        tokio::spawn(async move {
            shared_context.store(&name, truncated).await;
        });
    }
}

async fn forward_direct(
    state: &ProxyState,
    profile: &ProfileConfig,
    body: &Value,
    is_streaming: bool,
) -> anyhow::Result<Response> {
    let url = format!("{}/v1/messages", profile.base_url.trim_end_matches('/'));

    let mut req = state
        .http_client
        .post(&url)
        .header("content-type", "application/json")
        .header("anthropic-version", "2023-06-01");

    if !profile.api_key.is_empty() {
        req = req.header("x-api-key", &profile.api_key);
    }

    for (k, v) in &profile.custom_headers {
        req = req.header(k.as_str(), v.as_str());
    }

    req = req.json(body);

    let resp = req.send().await?;
    let status = resp.status();

    if is_streaming {
        let stream = resp.bytes_stream();
        let response = Response::builder()
            .status(status.as_u16())
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .body(Body::from_stream(stream))
            .map_err(|e| anyhow::anyhow!("failed to build response: {e}"))?;
        Ok(response)
    } else {
        let resp_bytes = resp.bytes().await?;
        // Store context from non-streaming Anthropic-format response
        if let Ok(resp_json) = serde_json::from_slice::<Value>(&resp_bytes) {
            extract_and_store_context(state, &profile.name, &resp_json);
        }
        let response = Response::builder()
            .status(status.as_u16())
            .header("content-type", "application/json")
            .body(Body::from(resp_bytes))
            .map_err(|e| anyhow::anyhow!("failed to build response: {e}"))?;
        Ok(response)
    }
}

async fn forward_translated(
    state: &ProxyState,
    profile: &ProfileConfig,
    body: &Value,
    is_streaming: bool,
) -> anyhow::Result<Response> {
    use super::translation;

    let openai_body = translation::anthropic_to_openai(body, &profile.default_model)?;

    let url = format!(
        "{}/chat/completions",
        profile.base_url.trim_end_matches('/')
    );

    let mut req = state
        .http_client
        .post(&url)
        .header("content-type", "application/json");

    if !profile.api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", profile.api_key));
    }

    for (k, v) in &profile.custom_headers {
        req = req.header(k.as_str(), v.as_str());
    }

    req = req.json(&openai_body);

    let resp = req.send().await?;
    let status = resp.status();

    if !status.is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        anyhow::bail!("upstream returned HTTP {status}: {err_body}");
    }

    if is_streaming {
        let stream = resp.bytes_stream();
        let translated_stream = super::streaming::translate_sse_stream(stream);
        let response = Response::builder()
            .status(200)
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .body(Body::from_stream(translated_stream))
            .map_err(|e| anyhow::anyhow!("failed to build response: {e}"))?;
        Ok(response)
    } else {
        let openai_resp: Value = resp.json().await?;
        let anthropic_resp = translation::openai_to_anthropic(&openai_resp)?;
        // Store context from non-streaming translated response
        extract_and_store_context(state, &profile.name, &anthropic_resp);
        let response = Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&anthropic_resp)?))
            .map_err(|e| anyhow::anyhow!("failed to build response: {e}"))?;
        Ok(response)
    }
}
