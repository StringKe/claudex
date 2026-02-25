use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

use crate::config::ProviderType;
use crate::proxy::ProxyState;

pub async fn list_models(State(state): State<Arc<ProxyState>>) -> impl IntoResponse {
    let config = state.config.read().await;
    let profiles = config.enabled_profiles();

    let mut models = Vec::new();

    for profile in profiles {
        // Always include the default model
        models.push(json!({
            "id": profile.default_model,
            "object": "model",
            "created": 0,
            "owned_by": profile.name,
            "x-claudex-profile": profile.name,
            "x-claudex-provider": match profile.provider_type {
                ProviderType::DirectAnthropic => "anthropic",
                ProviderType::OpenAICompatible => "openai-compatible",
                ProviderType::OpenAIResponses => "openai-responses",
            },
        }));
    }

    (
        StatusCode::OK,
        Json(json!({
            "object": "list",
            "data": models,
        })),
    )
}
