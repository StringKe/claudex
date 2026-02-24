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

    pub async fn gather_for_profile(&self, target_profile: &str, config: &SharingConfig) -> String {
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
                context_parts.push(format!("[From {}] {}", entry.source_profile, entry.content));
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_key_info_short_response_ignored() {
        let body = json!({
            "messages": [{"role": "assistant", "content": "short"}]
        });
        assert!(extract_key_info(&body).is_none());
    }

    #[test]
    fn test_extract_key_info_long_response() {
        let long_text = "x".repeat(150);
        let body = json!({
            "messages": [{"role": "assistant", "content": long_text}]
        });
        let result = extract_key_info(&body).unwrap();
        assert_eq!(result, long_text);
    }

    #[test]
    fn test_extract_key_info_truncates_at_500() {
        let long_text = "x".repeat(600);
        let body = json!({
            "messages": [{"role": "assistant", "content": long_text}]
        });
        let result = extract_key_info(&body).unwrap();
        assert_eq!(result.len(), 503); // 500 + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_extract_key_info_not_assistant() {
        let body = json!({
            "messages": [{"role": "user", "content": "x".repeat(200)}]
        });
        assert!(extract_key_info(&body).is_none());
    }

    #[test]
    fn test_extract_key_info_empty_messages() {
        let body = json!({"messages": []});
        assert!(extract_key_info(&body).is_none());
    }

    #[test]
    fn test_extract_key_info_array_content() {
        let long_text = "y".repeat(200);
        let body = json!({
            "messages": [{
                "role": "assistant",
                "content": [{"type": "text", "text": long_text}]
            }]
        });
        let result = extract_key_info(&body).unwrap();
        assert_eq!(result, long_text);
    }

    #[tokio::test]
    async fn test_shared_context_store_and_gather() {
        let ctx = SharedContext::new();
        ctx.store("profile_a", "info from a".to_string()).await;
        ctx.store("profile_b", "info from b".to_string()).await;

        let config = SharingConfig {
            enabled: true,
            max_context_size: 10000,
        };

        // Gathering for profile_a should NOT include profile_a's own data
        let gathered = ctx.gather_for_profile("profile_a", &config).await;
        assert!(gathered.contains("info from b"));
        assert!(!gathered.contains("[From profile_a]"));
    }

    #[tokio::test]
    async fn test_shared_context_disabled() {
        let ctx = SharedContext::new();
        ctx.store("profile_a", "data".to_string()).await;

        let config = SharingConfig {
            enabled: false,
            max_context_size: 10000,
        };
        let gathered = ctx.gather_for_profile("profile_b", &config).await;
        assert!(gathered.is_empty());
    }

    #[tokio::test]
    async fn test_shared_context_size_limit() {
        let ctx = SharedContext::new();
        ctx.store("a", "x".repeat(100)).await;
        ctx.store("a", "y".repeat(100)).await;

        let config = SharingConfig {
            enabled: true,
            max_context_size: 120, // only room for ~1 entry
        };
        let gathered = ctx.gather_for_profile("b", &config).await;
        // Should have at most the entries that fit within 120 chars
        assert!(gathered.len() <= 150); // some overhead from "[From a] " prefix
    }
}
