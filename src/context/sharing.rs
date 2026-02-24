use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::RwLock;

use super::SharingConfig;

#[derive(Debug, Clone)]
pub struct SharedContext {
    inner: Arc<RwLock<HashMap<String, Vec<ContextEntry>>>>,
}

#[derive(Debug, Clone)]
pub struct ContextEntry {
    pub source_profile: String,
    pub content: String,
    pub timestamp: std::time::Instant,
}

impl SharedContext {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn store(&self, profile: &str, content: String) {
        let mut map = self.inner.write().await;
        let entries = map.entry(profile.to_string()).or_default();
        entries.push(ContextEntry {
            source_profile: profile.to_string(),
            content,
            timestamp: std::time::Instant::now(),
        });

        // Keep only last 50 entries per profile
        if entries.len() > 50 {
            entries.drain(..entries.len() - 50);
        }
    }

    pub async fn gather_for_profile(
        &self,
        target_profile: &str,
        config: &SharingConfig,
    ) -> String {
        if !config.enabled {
            return String::new();
        }

        let map = self.inner.read().await;
        let mut context_parts = Vec::new();
        let mut total_size = 0;

        for (profile, entries) in map.iter() {
            if profile == target_profile {
                continue;
            }
            for entry in entries.iter().rev() {
                if total_size + entry.content.len() > config.max_context_size {
                    break;
                }
                context_parts.push(format!(
                    "[From {}] {}",
                    entry.source_profile, entry.content
                ));
                total_size += entry.content.len();
            }
        }

        context_parts.join("\n\n")
    }
}

pub fn extract_key_info(body: &Value) -> Option<String> {
    let messages = body.get("messages")?.as_array()?;
    let last = messages.last()?;

    if last.get("role")?.as_str()? != "assistant" {
        return None;
    }

    let content = match last.get("content") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => return None,
    };

    // Only share substantial responses
    if content.len() < 100 {
        return None;
    }

    // Truncate to reasonable size
    let truncated = if content.len() > 500 {
        format!("{}...", &content[..500])
    } else {
        content
    };

    Some(truncated)
}
