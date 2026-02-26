mod chat_completions;
mod direct;
mod responses;

use anyhow::Result;
use bytes::Bytes;
use futures::stream::Stream;
use reqwest::RequestBuilder;
use serde_json::Value;
use std::pin::Pin;

use crate::config::{ProfileConfig, ProviderType};
use crate::proxy::util::ToolNameMap;

pub struct TranslatedRequest {
    pub body: Value,
    pub tool_name_map: ToolNameMap,
}

pub type ByteStream = Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>;

pub trait ProviderAdapter: Send + Sync {
    /// API 端点路径（如 "/v1/messages"、"/chat/completions"）
    fn endpoint_path(&self) -> &str;

    /// 将 Anthropic 请求翻译为目标格式
    fn translate_request(&self, body: &Value, profile: &ProfileConfig)
        -> Result<TranslatedRequest>;

    /// 根据 profile 的 strip_params 配置过滤翻译后的请求体
    fn filter_translated_body(&self, body: &mut Value, profile: &ProfileConfig) {
        let params_to_strip = profile.strip_params.resolve(&profile.base_url);
        if let Some(obj) = body.as_object_mut() {
            for param in &params_to_strip {
                if obj.remove(param).is_some() {
                    tracing::debug!(
                        "stripped unsupported param '{}' for {}",
                        param,
                        profile.name
                    );
                }
            }
        }
    }

    /// 设置认证头
    fn apply_auth(&self, builder: RequestBuilder, profile: &ProfileConfig) -> RequestBuilder;

    /// 设置额外头（如 ChatGPT-Account-ID），默认无操作
    fn apply_extra_headers(
        &self,
        builder: RequestBuilder,
        _profile: &ProfileConfig,
    ) -> RequestBuilder {
        builder
    }

    /// 是否直接透传上游响应（不做错误翻译和响应翻译）
    fn passthrough(&self) -> bool {
        false
    }

    /// 翻译非流式响应
    fn translate_response(&self, body: &Value, tool_name_map: &ToolNameMap) -> Result<Value>;

    /// 翻译流式响应
    fn translate_stream(&self, stream: ByteStream, tool_name_map: ToolNameMap) -> ByteStream;
}

/// 根据 ProviderType 创建对应的 Adapter
pub fn for_provider(provider_type: &ProviderType) -> Box<dyn ProviderAdapter> {
    match provider_type {
        ProviderType::DirectAnthropic => Box::new(direct::DirectAnthropicAdapter),
        ProviderType::OpenAICompatible => Box::new(chat_completions::ChatCompletionsAdapter),
        ProviderType::OpenAIResponses => Box::new(responses::ResponsesAdapter),
    }
}
