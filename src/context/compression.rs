use anyhow::Result;
use serde_json::{json, Value};

const COMPRESSION_PROMPT: &str = r#"You are a conversation summarizer. Compress the following conversation history into a concise summary that preserves:
1. Key decisions and conclusions
2. Important code snippets or file paths mentioned
3. Current task context and progress
4. Any constraints or requirements stated

Output a brief but comprehensive summary."#;

pub async fn compress_messages(
    enabled: bool,
    keep_recent: usize,
    base_url: &str,
    api_key: &str,
    model: &str,
    messages: &[Value],
    http_client: &reqwest::Client,
) -> Result<Value> {
    if !enabled || messages.len() <= keep_recent {
        return Ok(json!(messages));
    }

    let split_at = messages.len().saturating_sub(keep_recent);
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

    let summary =
        call_summarizer(base_url, api_key, model, &conversation_text, http_client).await?;

    let mut result = vec![json!({
        "role": "user",
        "content": format!("[Previous conversation summary]\n{summary}")
    })];
    result.extend(recent_messages.iter().cloned());

    Ok(json!(result))
}

async fn call_summarizer(
    base_url: &str,
    api_key: &str,
    model: &str,
    text: &str,
    http_client: &reqwest::Client,
) -> Result<String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": COMPRESSION_PROMPT},
            {"role": "user", "content": text},
        ],
        "max_tokens": 1000,
        "temperature": 0.3,
    });

    let mut req = http_client.post(&url).json(&body);
    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {api_key}"));
    }

    let resp: Value = req.send().await?.json().await?;

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
