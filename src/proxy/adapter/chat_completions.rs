use anyhow::Result;
use reqwest::RequestBuilder;
use serde_json::Value;

use super::{ByteStream, ProviderAdapter, TranslatedRequest};
use crate::config::ProfileConfig;
use crate::proxy::util::ToolNameMap;

pub struct ChatCompletionsAdapter;

impl ProviderAdapter for ChatCompletionsAdapter {
    fn endpoint_path(&self) -> &str {
        "/chat/completions"
    }

    fn translate_request(
        &self,
        body: &Value,
        profile: &ProfileConfig,
    ) -> Result<TranslatedRequest> {
        let (openai_body, tool_name_map) =
            crate::proxy::translate::chat_completions::anthropic_to_openai(
                body,
                &profile.default_model,
                profile.max_tokens,
            )?;
        Ok(TranslatedRequest {
            body: openai_body,
            tool_name_map,
        })
    }

    fn apply_auth(&self, builder: RequestBuilder, profile: &ProfileConfig) -> RequestBuilder {
        if !profile.api_key.is_empty() {
            if profile.extra_env.contains_key("AZURE_AUTH")
                || profile.base_url.contains("openai.azure.com")
            {
                builder.header("api-key", &profile.api_key)
            } else {
                builder.header("Authorization", format!("Bearer {}", profile.api_key))
            }
        } else {
            builder
        }
    }

    fn apply_extra_headers(
        &self,
        mut builder: RequestBuilder,
        profile: &ProfileConfig,
    ) -> RequestBuilder {
        // GitHub Copilot: 添加伪装 headers
        if profile.extra_env.contains_key("COPILOT_AUTH")
            || profile.base_url.contains("githubcopilot.com")
        {
            for (k, v) in crate::oauth::exchange::copilot_extra_headers() {
                builder = builder.header(k, v);
            }
        }
        builder
    }

    fn translate_response(&self, body: &Value, tool_name_map: &ToolNameMap) -> Result<Value> {
        crate::proxy::translate::chat_completions::openai_to_anthropic(body, tool_name_map)
    }

    fn translate_stream(&self, stream: ByteStream, tool_name_map: ToolNameMap) -> ByteStream {
        crate::proxy::translate::chat_completions_stream::translate_sse_stream(
            stream,
            tool_name_map,
        )
    }
}
