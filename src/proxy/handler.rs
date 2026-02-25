use std::sync::Arc;
use std::time::Instant;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::Value;

use crate::config::ProfileConfig;
use crate::oauth::AuthType;
use crate::proxy::ProxyState;
use crate::router::classifier;

pub async fn handle_messages(
    State(state): State<Arc<ProxyState>>,
    Path(profile_name): Path<String>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let start = Instant::now();

    // 入站请求日志
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            if s.len() > 20 {
                format!("{}...", &s[..20])
            } else {
                s.to_string()
            }
        })
        .unwrap_or_else(|| "(none)".to_string());
    let api_key_header = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            if s.len() > 20 {
                format!("{}...", &s[..20])
            } else {
                s.to_string()
            }
        })
        .unwrap_or_else(|| "(none)".to_string());

    tracing::info!(
        profile = %profile_name,
        authorization = %auth_header,
        x_api_key = %api_key_header,
        body_len = %body.len(),
        "incoming request"
    );

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

    let mut profile = match config.find_profile(&resolved_profile_name) {
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

    // OAuth token lazy refresh
    if profile.auth_type == AuthType::OAuth {
        if let Err(e) = crate::oauth::providers::ensure_valid_token(&mut profile).await {
            return (StatusCode::UNAUTHORIZED, format!("OAuth token error: {e}")).into_response();
        }
    }

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

    match classifier::classify_intent(
        &base_url,
        &api_key,
        &model,
        &user_message,
        &state.http_client,
    )
    .await
    {
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
/// Uses ProviderAdapter trait to handle provider-specific translation.
async fn try_forward(
    state: &ProxyState,
    profile: &ProfileConfig,
    _headers: &HeaderMap,
    body: &Value,
    is_streaming: bool,
) -> anyhow::Result<Response> {
    let adapter = super::adapter::for_provider(&profile.provider_type);
    let translated = adapter.translate_request(body, profile)?;

    let url = format!(
        "{}{}",
        profile.base_url.trim_end_matches('/'),
        adapter.endpoint_path()
    );
    let key_preview = super::util::format_key_preview(&profile.api_key);

    tracing::info!(
        profile = %profile.name,
        url = %url,
        api_key = %key_preview,
        streaming = %is_streaming,
        model = %translated.body.get("model").and_then(|v| v.as_str()).unwrap_or("-"),
        "forwarding request"
    );

    let mut req = state
        .http_client
        .post(&url)
        .header("content-type", "application/json");

    req = adapter.apply_auth(req, profile);
    req = adapter.apply_extra_headers(req, profile);

    for (k, v) in &profile.custom_headers {
        req = req.header(k.as_str(), v.as_str());
    }

    req = req.json(&translated.body);

    let resp = req.send().await?;
    let status = resp.status();

    tracing::info!(
        profile = %profile.name,
        status = %status,
        "upstream response"
    );

    if adapter.passthrough() {
        // Direct passthrough (e.g., DirectAnthropic): no error/response translation
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
    } else {
        // Translated path: handle errors, then translate response
        if !status.is_success() {
            let err_body = resp.text().await.unwrap_or_default();

            if status.is_client_error() {
                // 4xx: non-retryable, translate to Anthropic error format
                tracing::warn!(
                    profile = %profile.name,
                    status = %status,
                    body = %err_body,
                    "client error (non-retryable)"
                );
                let anthropic_err = super::util::to_anthropic_error(status.as_u16(), &err_body);
                let response = Response::builder()
                    .status(status.as_u16())
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&anthropic_err).unwrap_or_default(),
                    ))
                    .map_err(|e| anyhow::anyhow!("failed to build error response: {e}"))?;
                return Ok(response);
            }

            // 5xx: retryable, bail for circuit breaker + failover
            tracing::error!(
                profile = %profile.name,
                status = %status,
                body = %err_body,
                "upstream error"
            );
            anyhow::bail!("upstream returned HTTP {status}: {err_body}");
        }

        if is_streaming {
            let stream = resp.bytes_stream();
            let translated_stream =
                adapter.translate_stream(Box::pin(stream), translated.tool_name_map);
            let response = Response::builder()
                .status(200)
                .header("content-type", "text/event-stream")
                .header("cache-control", "no-cache")
                .body(Body::from_stream(translated_stream))
                .map_err(|e| anyhow::anyhow!("failed to build response: {e}"))?;
            Ok(response)
        } else {
            let resp_json: Value = resp.json().await?;
            let anthropic_resp =
                adapter.translate_response(&resp_json, &translated.tool_name_map)?;
            extract_and_store_context(state, &profile.name, &anthropic_resp);
            let response = Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&anthropic_resp)?))
                .map_err(|e| anyhow::anyhow!("failed to build response: {e}"))?;
            Ok(response)
        }
    }
}

/// Extract assistant text from an Anthropic-format response and store for sharing.
fn extract_and_store_context(state: &ProxyState, profile_name: &str, resp_body: &Value) {
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
