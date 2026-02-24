use std::sync::Arc;
use std::time::Instant;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::config::ProviderType;
use crate::proxy::ProxyState;

pub async fn handle_messages(
    State(state): State<Arc<ProxyState>>,
    Path(profile_name): Path<String>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let start = Instant::now();
    let config = state.config.read().await;

    let profile = match config.find_profile(&profile_name) {
        Some(p) => p.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                format!("profile '{profile_name}' not found"),
            )
                .into_response();
        }
    };

    if !profile.enabled {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            format!("profile '{profile_name}' is disabled"),
        )
            .into_response();
    }

    let metrics = state.metrics.get_or_create(&profile_name);
    drop(config);

    let body_value: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")).into_response();
        }
    };

    let is_streaming = body_value
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let result = match profile.provider_type {
        ProviderType::DirectAnthropic => {
            forward_direct(&state, &profile, &headers, &body_value, is_streaming).await
        }
        ProviderType::OpenAICompatible => {
            forward_translated(&state, &profile, &headers, &body_value, is_streaming).await
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
            tracing::error!(profile = %profile_name, error = %e, "proxy request failed");
            (StatusCode::BAD_GATEWAY, format!("proxy error: {e}")).into_response()
        }
    }
}

async fn forward_direct(
    state: &ProxyState,
    profile: &crate::config::ProfileConfig,
    _headers: &HeaderMap,
    body: &serde_json::Value,
    is_streaming: bool,
) -> anyhow::Result<Response> {
    let url = format!(
        "{}/v1/messages",
        profile.base_url.trim_end_matches('/')
    );

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
        Ok(Response::builder()
            .status(status.as_u16())
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .body(Body::from_stream(stream))
            .unwrap())
    } else {
        let resp_bytes = resp.bytes().await?;
        Ok(Response::builder()
            .status(status.as_u16())
            .header("content-type", "application/json")
            .body(Body::from(resp_bytes))
            .unwrap())
    }
}

async fn forward_translated(
    state: &ProxyState,
    profile: &crate::config::ProfileConfig,
    _headers: &HeaderMap,
    body: &serde_json::Value,
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
        Ok(Response::builder()
            .status(200)
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .body(Body::from_stream(translated_stream))
            .unwrap())
    } else {
        let openai_resp: serde_json::Value = resp.json().await?;
        let anthropic_resp = translation::openai_to_anthropic(&openai_resp)?;
        Ok(Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&anthropic_resp)?))
            .unwrap())
    }
}
