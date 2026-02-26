use serde_json::Value;

use crate::config::ClaudexConfig;
use crate::context::resolve_profile_endpoint;
use crate::context::ContextEngineConfig;
use crate::proxy::ProxyState;
use crate::router::classifier;

/// Apply context engine pre-processing to the request body.
/// This handles RAG injection, context sharing, and conversation compression.
pub async fn apply_context_engine(
    body: &mut Value,
    state: &ProxyState,
    profile: &str,
    context_config: &ContextEngineConfig,
    config: &ClaudexConfig,
) {
    // 1. RAG injection
    if context_config.rag.enabled {
        if let Some(ref rag_index) = state.rag_index {
            inject_rag_context(body, rag_index, state, config, context_config).await;
        }
    }

    // 2. Cross-profile context sharing
    if context_config.sharing.enabled {
        let shared = state
            .shared_context
            .gather_for_profile(profile, &context_config.sharing)
            .await;
        if !shared.is_empty() {
            inject_system_context(
                body,
                &format!("[Shared context from other profiles]\n{shared}"),
            );
        }
    }

    // 3. Conversation compression
    if context_config.compression.enabled {
        compress_if_needed(
            body,
            &context_config.compression,
            &state.http_client,
            config,
        )
        .await;
    }
}

/// Extract query from body and inject RAG results into system prompt
async fn inject_rag_context(
    body: &mut Value,
    rag_index: &crate::context::rag::RagIndex,
    state: &ProxyState,
    config: &ClaudexConfig,
    context_config: &ContextEngineConfig,
) {
    let query = classifier::extract_last_user_message(body).unwrap_or_default();
    if query.is_empty() {
        return;
    }

    let endpoint = resolve_profile_endpoint(
        config,
        &context_config.rag.profile,
        &context_config.rag.model,
    );
    let (base_url, api_key) = match endpoint {
        Some((url, key, _)) => (url, key),
        None => {
            tracing::debug!(
                profile = %context_config.rag.profile,
                "RAG profile not found, skipping search"
            );
            return;
        }
    };

    match rag_index
        .search(&query, &state.http_client, &base_url, &api_key)
        .await
    {
        Ok(results) if !results.is_empty() => {
            let rag_context = format!("[Relevant code context]\n{}", results.join("\n---\n"));
            inject_system_context(body, &rag_context);
        }
        Ok(_) => {}
        Err(e) => {
            tracing::debug!(error = %e, "RAG search failed");
        }
    }
}

/// Inject additional context into the system prompt
fn inject_system_context(body: &mut Value, context: &str) {
    match body.get("system") {
        Some(Value::String(existing)) => {
            body["system"] = Value::String(format!("{existing}\n\n{context}"));
        }
        Some(Value::Array(parts)) => {
            let existing: String = parts
                .iter()
                .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("\n");
            body["system"] = Value::String(format!("{existing}\n\n{context}"));
        }
        _ => {
            body["system"] = Value::String(context.to_string());
        }
    }
}

/// Compress conversation if it exceeds the token threshold
async fn compress_if_needed(
    body: &mut Value,
    compression: &crate::context::CompressionConfig,
    http_client: &reqwest::Client,
    config: &ClaudexConfig,
) {
    let messages = match body.get("messages").and_then(|m| m.as_array()) {
        Some(m) => m.clone(),
        None => return,
    };

    // Rough token estimation: ~4 chars per token
    let total_chars: usize = messages.iter().map(|m| m.to_string().len()).sum();
    let estimated_tokens = total_chars / 4;

    if estimated_tokens <= compression.threshold_tokens {
        return;
    }

    let endpoint = resolve_profile_endpoint(config, &compression.profile, &compression.model);
    let (base_url, api_key, model) = match endpoint {
        Some(v) => v,
        None => {
            tracing::warn!(
                profile = %compression.profile,
                "compression profile not found, skipping"
            );
            return;
        }
    };

    match crate::context::compression::compress_messages(
        compression.enabled,
        compression.keep_recent,
        &base_url,
        &api_key,
        &model,
        &messages,
        http_client,
    )
    .await
    {
        Ok(compressed) => {
            body["messages"] = compressed;
            tracing::info!(
                original_messages = messages.len(),
                estimated_tokens,
                "compressed conversation history"
            );
        }
        Err(e) => {
            tracing::warn!(error = %e, "conversation compression failed");
        }
    }
}
