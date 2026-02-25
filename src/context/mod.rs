pub mod compression;
pub mod rag;
pub mod sharing;

use serde::{Deserialize, Serialize};

use crate::config::ClaudexConfig;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextEngineConfig {
    #[serde(default)]
    pub compression: CompressionConfig,
    #[serde(default)]
    pub sharing: SharingConfig,
    #[serde(default)]
    pub rag: RagConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_threshold_tokens")]
    pub threshold_tokens: usize,
    #[serde(default = "default_keep_recent")]
    pub keep_recent: usize,
    #[serde(default)]
    pub profile: String,
    #[serde(default)]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_max_context_size")]
    pub max_context_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub index_paths: Vec<String>,
    #[serde(default)]
    pub profile: String,
    #[serde(default)]
    pub model: String,
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
}

fn default_threshold_tokens() -> usize {
    50000
}
fn default_keep_recent() -> usize {
    10
}
fn default_max_context_size() -> usize {
    2000
}
fn default_chunk_size() -> usize {
    512
}
fn default_top_k() -> usize {
    5
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_tokens: default_threshold_tokens(),
            keep_recent: default_keep_recent(),
            profile: String::new(),
            model: String::new(),
        }
    }
}

impl Default for SharingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_context_size: default_max_context_size(),
        }
    }
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            index_paths: Vec::new(),
            profile: String::new(),
            model: String::new(),
            chunk_size: default_chunk_size(),
            top_k: default_top_k(),
        }
    }
}

/// Resolve a profile reference to (base_url, api_key, model).
/// `model_override` takes precedence over the profile's `default_model`.
pub fn resolve_profile_endpoint(
    config: &ClaudexConfig,
    profile_name: &str,
    model_override: &str,
) -> Option<(String, String, String)> {
    let p = config.find_profile(profile_name)?;
    let model = if model_override.is_empty() {
        &p.default_model
    } else {
        model_override
    };
    Some((p.base_url.clone(), p.api_key.clone(), model.to_string()))
}
