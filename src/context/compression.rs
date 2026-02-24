use anyhow::Result;
use serde_json::{json, Value};

use super::CompressionConfig;

const COMPRESSION_PROMPT: &str = r#"You are a conversation summarizer. Compress the following conversation history into a concise summary that preserves:
1. Key decisions and conclusions
2. Important code snippets or file paths mentioned
3. Current task context and progress
4. Any constraints or requirements stated

Output a brief but comprehensive summary."#;

pub async fn compress_messages(
    config: &CompressionConfig,
    messages: &[Value],
    http_client: &reqwest::Client,
) -> Result<Value> {
    if !config.enabled || messages.len() <= config.keep_recent {
        return Ok(json!(messages));
    }

    let split_at = messages.len().saturating_sub(config.keep_recent);
    let old_messages = &messages[..split_at];
    let recent_messages = &messages[split_at..];

    // Build conversation text from old messages
    let conversation_text: String = old_messages
        .iter()
        .filter_map(|msg| {
            let role = msg.get("role")?.as_str()?;
            let content = msg.get("content")?.as_str()?;
            Some(format!("{role}: {content}"))
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    if conversation_text.is_empty() {
        return Ok(json!(messages));
    }

    let summary = call_summarizer(config, &conversation_text, http_client).await?;

    let mut result = vec![json!({
        "role": "user",
        "content": format!("[Previous conversation summary]\n{summary}")
    })];
    result.extend(recent_messages.iter().cloned());

    Ok(json!(result))
}

async fn call_summarizer(
    config: &CompressionConfig,
    text: &str,
    http_client: &reqwest::Client,
) -> Result<String> {
    let url = format!(
        "{}/chat/completions",
        config.summarizer_url.trim_end_matches('/')
    );

    let body = json!({
        "model": config.summarizer_model,
        "messages": [
            {"role": "system", "content": COMPRESSION_PROMPT},
            {"role": "user", "content": text},
        ],
        "max_tokens": 1000,
        "temperature": 0.3,
    });

    let resp: Value = http_client.post(&url).json(&body).send().await?.json().await?;

    let summary = resp
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|c| c.first())
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("[compression failed]")
        .to_string();

    Ok(summary)
}
