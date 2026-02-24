use anyhow::Result;
use serde_json::{json, Value};

/// Convert Anthropic Messages API request → OpenAI Chat Completions request
pub fn anthropic_to_openai(anthropic: &Value, default_model: &str) -> Result<Value> {
    let mut messages = Vec::new();

    // System prompt → system message
    if let Some(system) = anthropic.get("system") {
        let system_text = match system {
            Value::String(s) => s.clone(),
            Value::Array(parts) => parts
                .iter()
                .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("\n"),
            _ => String::new(),
        };
        if !system_text.is_empty() {
            messages.push(json!({"role": "system", "content": system_text}));
        }
    }

    // Convert messages
    if let Some(msgs) = anthropic.get("messages").and_then(|m| m.as_array()) {
        for msg in msgs {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
            let content = convert_content_to_openai(msg.get("content"));

            match role {
                "tool" => {
                    // Anthropic tool_result → OpenAI tool message
                    let tool_use_id = msg
                        .get("tool_use_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": tool_use_id,
                        "content": content_to_string(&content),
                    }));
                }
                "assistant" => {
                    let mut assistant_msg = json!({"role": "assistant"});

                    // Check for tool_use blocks in content
                    if let Some(content_arr) = msg.get("content").and_then(|c| c.as_array()) {
                        let mut text_parts = Vec::new();
                        let mut tool_calls = Vec::new();

                        for block in content_arr {
                            match block.get("type").and_then(|t| t.as_str()) {
                                Some("text") => {
                                    if let Some(text) = block.get("text").and_then(|t| t.as_str())
                                    {
                                        text_parts.push(text.to_string());
                                    }
                                }
                                Some("tool_use") => {
                                    tool_calls.push(json!({
                                        "id": block.get("id").unwrap_or(&json!("")),
                                        "type": "function",
                                        "function": {
                                            "name": block.get("name").unwrap_or(&json!("")),
                                            "arguments": serde_json::to_string(
                                                block.get("input").unwrap_or(&json!({}))
                                            ).unwrap_or_default(),
                                        }
                                    }));
                                }
                                _ => {}
                            }
                        }

                        if !text_parts.is_empty() {
                            assistant_msg["content"] = json!(text_parts.join("\n"));
                        }
                        if !tool_calls.is_empty() {
                            assistant_msg["tool_calls"] = json!(tool_calls);
                        }
                    } else {
                        assistant_msg["content"] = content;
                    }

                    messages.push(assistant_msg);
                }
                _ => {
                    messages.push(json!({
                        "role": role,
                        "content": content,
                    }));
                }
            }
        }
    }

    let model = anthropic
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or(default_model);

    let mut openai_req = json!({
        "model": model,
        "messages": messages,
    });

    // Forward simple parameters
    if let Some(max_tokens) = anthropic.get("max_tokens") {
        openai_req["max_tokens"] = max_tokens.clone();
    }
    if let Some(temperature) = anthropic.get("temperature") {
        openai_req["temperature"] = temperature.clone();
    }
    if let Some(top_p) = anthropic.get("top_p") {
        openai_req["top_p"] = top_p.clone();
    }
    if let Some(stream) = anthropic.get("stream") {
        openai_req["stream"] = stream.clone();
    }

    // Convert tools
    if let Some(tools) = anthropic.get("tools").and_then(|t| t.as_array()) {
        let openai_tools: Vec<Value> = tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.get("name").unwrap_or(&json!("")),
                        "description": tool.get("description").unwrap_or(&json!("")),
                        "parameters": tool.get("input_schema").unwrap_or(&json!({})),
                    }
                })
            })
            .collect();
        openai_req["tools"] = json!(openai_tools);
    }

    // Convert tool_choice
    if let Some(tc) = anthropic.get("tool_choice") {
        openai_req["tool_choice"] = convert_tool_choice(tc);
    }

    Ok(openai_req)
}

/// Convert OpenAI Chat Completions response → Anthropic Messages API response
pub fn openai_to_anthropic(openai: &Value) -> Result<Value> {
    let empty_obj = json!({});
    let choice = openai
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|c| c.first())
        .unwrap_or(&empty_obj);

    let message = choice.get("message").unwrap_or(&empty_obj);

    let mut content = Vec::new();

    // Text content
    if let Some(text) = message.get("content").and_then(|c| c.as_str()) {
        if !text.is_empty() {
            content.push(json!({
                "type": "text",
                "text": text,
            }));
        }
    }

    // Tool calls
    if let Some(tool_calls) = message.get("tool_calls").and_then(|tc| tc.as_array()) {
        for tc in tool_calls {
            let empty_func = json!({});
            let func = tc.get("function").unwrap_or(&empty_func);
            let args_str = func
                .get("arguments")
                .and_then(|a| a.as_str())
                .unwrap_or("{}");
            let input: Value = serde_json::from_str(args_str).unwrap_or(json!({}));

            content.push(json!({
                "type": "tool_use",
                "id": tc.get("id").unwrap_or(&json!("")),
                "name": func.get("name").unwrap_or(&json!("")),
                "input": input,
            }));
        }
    }

    // Stop reason mapping
    let finish_reason = choice
        .get("finish_reason")
        .and_then(|r| r.as_str())
        .unwrap_or("end_turn");
    let stop_reason = match finish_reason {
        "stop" => "end_turn",
        "tool_calls" => "tool_use",
        "length" => "max_tokens",
        "content_filter" => "end_turn",
        other => other,
    };

    // Usage
    let empty_usage = json!({});
    let usage = openai.get("usage").unwrap_or(&empty_usage);
    let input_tokens = usage
        .get("prompt_tokens")
        .and_then(|t| t.as_u64())
        .unwrap_or(0);
    let output_tokens = usage
        .get("completion_tokens")
        .and_then(|t| t.as_u64())
        .unwrap_or(0);

    let model = openai
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("unknown");

    let resp = json!({
        "id": openai.get("id").unwrap_or(&json!("msg_claudex")),
        "type": "message",
        "role": "assistant",
        "model": model,
        "content": content,
        "stop_reason": stop_reason,
        "stop_sequence": null,
        "usage": {
            "input_tokens": input_tokens,
            "output_tokens": output_tokens,
        }
    });

    Ok(resp)
}

fn convert_content_to_openai(content: Option<&Value>) -> Value {
    match content {
        None => json!(""),
        Some(Value::String(s)) => json!(s),
        Some(Value::Array(parts)) => {
            let openai_parts: Vec<Value> = parts
                .iter()
                .filter_map(|part| {
                    match part.get("type").and_then(|t| t.as_str()) {
                        Some("text") => Some(json!({
                            "type": "text",
                            "text": part.get("text").unwrap_or(&json!("")),
                        })),
                        Some("image") => {
                            let source = part.get("source")?;
                            Some(json!({
                                "type": "image_url",
                                "image_url": {
                                    "url": format!(
                                        "data:{};base64,{}",
                                        source.get("media_type").and_then(|m| m.as_str()).unwrap_or("image/png"),
                                        source.get("data").and_then(|d| d.as_str()).unwrap_or("")
                                    )
                                }
                            }))
                        }
                        Some("tool_result") => {
                            let result_content = part.get("content");
                            Some(json!({
                                "type": "text",
                                "text": content_to_string(&convert_content_to_openai(result_content)),
                            }))
                        }
                        _ => None,
                    }
                })
                .collect();

            if openai_parts.len() == 1 {
                if let Some(text) = openai_parts[0].get("text") {
                    return text.clone();
                }
            }
            json!(openai_parts)
        }
        Some(other) => other.clone(),
    }
}

fn content_to_string(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => content.to_string(),
    }
}

fn convert_tool_choice(tc: &Value) -> Value {
    match tc {
        Value::String(s) => match s.as_str() {
            "auto" => json!("auto"),
            "any" => json!("required"),
            "none" => json!("none"),
            _ => json!("auto"),
        },
        Value::Object(obj) => {
            if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
                json!({"type": "function", "function": {"name": name}})
            } else {
                json!("auto")
            }
        }
        _ => json!("auto"),
    }
}
