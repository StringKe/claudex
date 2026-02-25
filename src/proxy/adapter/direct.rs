use std::collections::HashMap;

use anyhow::Result;
use reqwest::RequestBuilder;
use serde_json::Value;

use super::{ByteStream, ProviderAdapter, TranslatedRequest};
use crate::config::ProfileConfig;
use crate::proxy::util::ToolNameMap;

pub struct DirectAnthropicAdapter;

impl ProviderAdapter for DirectAnthropicAdapter {
    fn endpoint_path(&self) -> &str {
        "/v1/messages"
    }

    fn translate_request(
        &self,
        body: &Value,
        _profile: &ProfileConfig,
    ) -> Result<TranslatedRequest> {
        Ok(TranslatedRequest {
            body: body.clone(),
            tool_name_map: HashMap::new(),
        })
    }

    fn apply_auth(&self, builder: RequestBuilder, profile: &ProfileConfig) -> RequestBuilder {
        let mut b = builder.header("anthropic-version", "2023-06-01");
        if !profile.api_key.is_empty() {
            b = b.header("x-api-key", &profile.api_key);
        }
        b
    }

    fn passthrough(&self) -> bool {
        true
    }

    fn translate_response(&self, body: &Value, _tool_name_map: &ToolNameMap) -> Result<Value> {
        Ok(body.clone())
    }

    fn translate_stream(&self, stream: ByteStream, _tool_name_map: ToolNameMap) -> ByteStream {
        stream
    }
}
