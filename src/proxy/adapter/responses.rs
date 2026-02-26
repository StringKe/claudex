use anyhow::Result;
use reqwest::RequestBuilder;
use serde_json::Value;

use super::{ByteStream, ProviderAdapter, TranslatedRequest};
use crate::config::ProfileConfig;
use crate::proxy::util::ToolNameMap;

pub struct ResponsesAdapter;

impl ProviderAdapter for ResponsesAdapter {
    fn endpoint_path(&self) -> &str {
        "/responses"
    }

    fn translate_request(
        &self,
        body: &Value,
        profile: &ProfileConfig,
    ) -> Result<TranslatedRequest> {
        let (responses_body, tool_name_map) =
            crate::proxy::translate::responses::anthropic_to_responses(
                body,
                &profile.default_model,
            )?;
        Ok(TranslatedRequest {
            body: responses_body,
            tool_name_map,
        })
    }

    fn apply_auth(&self, builder: RequestBuilder, profile: &ProfileConfig) -> RequestBuilder {
        if !profile.api_key.is_empty() {
            builder.header("Authorization", format!("Bearer {}", profile.api_key))
        } else {
            builder
        }
    }

    fn apply_extra_headers(
        &self,
        builder: RequestBuilder,
        profile: &ProfileConfig,
    ) -> RequestBuilder {
        if let Some(account_id) = profile.extra_env.get("CHATGPT_ACCOUNT_ID") {
            builder.header("ChatGPT-Account-ID", account_id.as_str())
        } else {
            builder
        }
    }

    fn translate_response(&self, body: &Value, tool_name_map: &ToolNameMap) -> Result<Value> {
        crate::proxy::translate::responses::responses_to_anthropic(body, tool_name_map)
    }

    fn translate_stream(&self, stream: ByteStream, tool_name_map: ToolNameMap) -> ByteStream {
        crate::proxy::translate::responses_stream::translate_responses_stream(stream, tool_name_map)
    }
}
