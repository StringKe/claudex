pub mod compression;
pub mod rag;
pub mod sharing;

use serde::{Deserialize, Serialize};

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
    #[serde(default = "default_summarizer_url")]
    pub summarizer_url: String,
    #[serde(default = "default_summarizer_model")]
    pub summarizer_model: String,
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
    #[serde(default = "default_embedding_url")]
    pub embedding_url: String,
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
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
fn default_summarizer_url() -> String {
    "http://localhost:11434/v1".to_string()
}
fn default_summarizer_model() -> String {
    "qwen2.5:3b".to_string()
}
fn default_max_context_size() -> usize {
    2000
}
fn default_embedding_url() -> String {
    "http://localhost:11434/v1".to_string()
}
fn default_embedding_model() -> String {
    "nomic-embed-text".to_string()
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
            summarizer_url: default_summarizer_url(),
            summarizer_model: default_summarizer_model(),
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
            embedding_url: default_embedding_url(),
            embedding_model: default_embedding_model(),
            chunk_size: default_chunk_size(),
            top_k: default_top_k(),
        }
    }
}
