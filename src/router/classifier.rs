use anyhow::Result;
use serde_json::{json, Value};

use super::RouterConfig;

const CLASSIFICATION_PROMPT: &str = r#"Analyze the following user request and classify its intent into exactly ONE of these categories:
- code: Code generation, modification, debugging, or programming tasks
- analysis: Code review, project analysis, architecture discussion
- creative: Creative writing, brainstorming, content generation
- search: Questions requiring up-to-date information or web search
- math: Mathematical reasoning, calculations, proofs

Respond with ONLY the category name, nothing else."#;

pub async fn classify_intent(
    config: &RouterConfig,
    prompt: &str,
    http_client: &reqwest::Client,
) -> Result<String> {
    let url = format!(
        "{}/chat/completions",
        config.classifier_url.trim_end_matches('/')
    );

    let body = json!({
        "model": config.classifier_model,
        "messages": [
            {"role": "system", "content": CLASSIFICATION_PROMPT},
            {"role": "user", "content": prompt},
        ],
        "max_tokens": 10,
        "temperature": 0.0,
    });

    let mut req = http_client.post(&url).json(&body);
    if !config.classifier_api_key.is_empty() {
        req = req.header(
            "Authorization",
            format!("Bearer {}", config.classifier_api_key),
        );
    }

    let resp: Value = req.send().await?.json().await?;

    let intent = resp
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|c| c.first())
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("default")
        .trim()
        .to_lowercase();

    Ok(intent)
}

pub fn extract_last_user_message(body: &Value) -> Option<String> {
    let messages = body.get("messages")?.as_array()?;
    for msg in messages.iter().rev() {
        if msg.get("role").and_then(|r| r.as_str()) == Some("user") {
            return match msg.get("content") {
                Some(Value::String(s)) => Some(s.clone()),
                Some(Value::Array(parts)) => {
                    let text: Vec<&str> = parts
                        .iter()
                        .filter_map(|p| {
                            if p.get("type").and_then(|t| t.as_str()) == Some("text") {
                                p.get("text").and_then(|t| t.as_str())
                            } else {
                                None
                            }
                        })
                        .collect();
                    Some(text.join("\n"))
                }
                _ => None,
            };
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_string_content() {
        let body = json!({
            "messages": [{"role": "user", "content": "hello world"}]
        });
        assert_eq!(
            extract_last_user_message(&body),
            Some("hello world".to_string())
        );
    }

    #[test]
    fn test_extract_array_content() {
        let body = json!({
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "part1"},
                    {"type": "image", "source": {}},
                    {"type": "text", "text": "part2"}
                ]
            }]
        });
        assert_eq!(
            extract_last_user_message(&body),
            Some("part1\npart2".to_string())
        );
    }

    #[test]
    fn test_extract_last_user_among_multiple() {
        let body = json!({
            "messages": [
                {"role": "user", "content": "first"},
                {"role": "assistant", "content": "reply"},
                {"role": "user", "content": "second"}
            ]
        });
        assert_eq!(extract_last_user_message(&body), Some("second".to_string()));
    }

    #[test]
    fn test_extract_no_user_message() {
        let body = json!({
            "messages": [{"role": "assistant", "content": "hi"}]
        });
        assert_eq!(extract_last_user_message(&body), None);
    }

    #[test]
    fn test_extract_empty_messages() {
        let body = json!({"messages": []});
        assert_eq!(extract_last_user_message(&body), None);
    }

    #[test]
    fn test_extract_no_messages_field() {
        let body = json!({"other": "field"});
        assert_eq!(extract_last_user_message(&body), None);
    }
}
