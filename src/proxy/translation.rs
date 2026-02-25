use std::collections::HashMap;

use anyhow::Result;
use serde_json::{json, Value};

use super::util::{truncate_tool_name, ToolNameMap};

/// Convert Anthropic Messages API request → OpenAI Chat Completions request
/// 返回 (openai_body, tool_name_map)，tool_name_map 用于在响应中还原被截断的工具名
pub fn anthropic_to_openai(anthropic: &Value, default_model: &str) -> Result<(Value, ToolNameMap)> {
    let mut tool_name_map: ToolNameMap = HashMap::new();
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
                                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                        text_parts.push(text.to_string());
                                    }
                                }
                                Some("tool_use") => {
                                    let orig =
                                        block.get("name").and_then(|n| n.as_str()).unwrap_or("");
                                    let truncated = truncate_tool_name(orig);
                                    if truncated != orig {
                                        tool_name_map.insert(truncated.clone(), orig.to_string());
                                    }
                                    tool_calls.push(json!({
                                        "id": block.get("id").unwrap_or(&json!("")),
                                        "type": "function",
                                        "function": {
                                            "name": truncated,
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

    // Convert tools（截断超过 64 字符的工具名）
    if let Some(tools) = anthropic.get("tools").and_then(|t| t.as_array()) {
        let openai_tools: Vec<Value> = tools
            .iter()
            .map(|tool| {
                let original_name = tool.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let truncated = truncate_tool_name(original_name);
                if truncated != original_name {
                    tool_name_map.insert(truncated.clone(), original_name.to_string());
                }
                json!({
                    "type": "function",
                    "function": {
                        "name": truncated,
                        "description": tool.get("description").unwrap_or(&json!("")),
                        "parameters": tool.get("input_schema").unwrap_or(&json!({})),
                    }
                })
            })
            .collect();
        openai_req["tools"] = json!(openai_tools);
    }

    // Convert tool_choice（工具名也需要截断）
    if let Some(tc) = anthropic.get("tool_choice") {
        openai_req["tool_choice"] = convert_tool_choice(tc, &tool_name_map);
    }

    if !tool_name_map.is_empty() {
        tracing::debug!(
            count = tool_name_map.len(),
            "truncated tool names for OpenAI compatibility"
        );
    }

    Ok((openai_req, tool_name_map))
}

/// Convert OpenAI Chat Completions response → Anthropic Messages API response
/// tool_name_map: 截断名 → 原始名，用于还原工具名
pub fn openai_to_anthropic(openai: &Value, tool_name_map: &ToolNameMap) -> Result<Value> {
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

    // Tool calls（还原被截断的工具名）
    if let Some(tool_calls) = message.get("tool_calls").and_then(|tc| tc.as_array()) {
        for tc in tool_calls {
            let empty_func = json!({});
            let func = tc.get("function").unwrap_or(&empty_func);
            let args_str = func
                .get("arguments")
                .and_then(|a| a.as_str())
                .unwrap_or("{}");
            let input: Value = serde_json::from_str(args_str).unwrap_or(json!({}));

            let truncated_name = func.get("name").and_then(|n| n.as_str()).unwrap_or("");
            // 还原原始名字（如果被截断过）
            let original_name = tool_name_map
                .get(truncated_name)
                .map(|s| s.as_str())
                .unwrap_or(truncated_name);

            content.push(json!({
                "type": "tool_use",
                "id": tc.get("id").unwrap_or(&json!("")),
                "name": original_name,
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

fn convert_tool_choice(tc: &Value, _tool_name_map: &ToolNameMap) -> Value {
    match tc {
        Value::String(s) => match s.as_str() {
            "auto" => json!("auto"),
            "any" => json!("required"),
            "none" => json!("none"),
            _ => json!("auto"),
        },
        Value::Object(obj) => {
            if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
                let truncated = truncate_tool_name(name);
                json!({"type": "function", "function": {"name": truncated}})
            } else {
                json!("auto")
            }
        }
        _ => json!("auto"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 辅助：调用 anthropic_to_openai 只取 body
    fn a2o(req: &Value, model: &str) -> Value {
        anthropic_to_openai(req, model).unwrap().0
    }

    /// 空映射
    fn empty_map() -> ToolNameMap {
        HashMap::new()
    }

    // --- anthropic_to_openai ---

    #[test]
    fn test_basic_user_message() {
        let req = json!({
            "messages": [{"role": "user", "content": "hello"}],
            "max_tokens": 100
        });
        let result = a2o(&req, "gpt-4");
        assert_eq!(result["model"], "gpt-4");
        assert_eq!(result["messages"][0]["role"], "user");
        assert_eq!(result["messages"][0]["content"], "hello");
        assert_eq!(result["max_tokens"], 100);
    }

    #[test]
    fn test_system_prompt_string() {
        let req = json!({
            "system": "You are helpful.",
            "messages": [{"role": "user", "content": "hi"}]
        });
        let result = a2o(&req, "m");
        assert_eq!(result["messages"][0]["role"], "system");
        assert_eq!(result["messages"][0]["content"], "You are helpful.");
        assert_eq!(result["messages"][1]["role"], "user");
    }

    #[test]
    fn test_system_prompt_array() {
        let req = json!({
            "system": [
                {"type": "text", "text": "Part 1"},
                {"type": "text", "text": "Part 2"}
            ],
            "messages": []
        });
        let result = a2o(&req, "m");
        assert_eq!(result["messages"][0]["content"], "Part 1\nPart 2");
    }

    #[test]
    fn test_model_override() {
        let req = json!({
            "model": "custom-model",
            "messages": [{"role": "user", "content": "hi"}]
        });
        let result = a2o(&req, "default-model");
        assert_eq!(result["model"], "custom-model");
    }

    #[test]
    fn test_parameters_passthrough() {
        let req = json!({
            "messages": [],
            "max_tokens": 500,
            "temperature": 0.7,
            "top_p": 0.9,
            "stream": true
        });
        let result = a2o(&req, "m");
        assert_eq!(result["max_tokens"], 500);
        assert_eq!(result["temperature"], 0.7);
        assert_eq!(result["top_p"], 0.9);
        assert_eq!(result["stream"], true);
    }

    #[test]
    fn test_assistant_with_tool_use() {
        let req = json!({
            "messages": [{
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "Let me search."},
                    {"type": "tool_use", "id": "call_1", "name": "search", "input": {"q": "rust"}}
                ]
            }]
        });
        let result = a2o(&req, "m");
        let msg = &result["messages"][0];
        assert_eq!(msg["role"], "assistant");
        assert_eq!(msg["content"], "Let me search.");
        assert_eq!(msg["tool_calls"][0]["id"], "call_1");
        assert_eq!(msg["tool_calls"][0]["type"], "function");
        assert_eq!(msg["tool_calls"][0]["function"]["name"], "search");
    }

    #[test]
    fn test_tool_result_message() {
        let req = json!({
            "messages": [{
                "role": "tool",
                "tool_use_id": "call_1",
                "content": "search result here"
            }]
        });
        let result = a2o(&req, "m");
        let msg = &result["messages"][0];
        assert_eq!(msg["role"], "tool");
        assert_eq!(msg["tool_call_id"], "call_1");
    }

    #[test]
    fn test_tools_conversion() {
        let req = json!({
            "messages": [],
            "tools": [{
                "name": "get_weather",
                "description": "Get weather info",
                "input_schema": {"type": "object", "properties": {"city": {"type": "string"}}}
            }]
        });
        let result = a2o(&req, "m");
        let tool = &result["tools"][0];
        assert_eq!(tool["type"], "function");
        assert_eq!(tool["function"]["name"], "get_weather");
        assert_eq!(tool["function"]["description"], "Get weather info");
        assert!(tool["function"]["parameters"]["properties"]["city"].is_object());
    }

    #[test]
    fn test_tool_choice_auto() {
        let req = json!({"messages": [], "tool_choice": "auto"});
        let result = a2o(&req, "m");
        assert_eq!(result["tool_choice"], "auto");
    }

    #[test]
    fn test_tool_choice_any() {
        let req = json!({"messages": [], "tool_choice": "any"});
        let result = a2o(&req, "m");
        assert_eq!(result["tool_choice"], "required");
    }

    #[test]
    fn test_tool_choice_none() {
        let req = json!({"messages": [], "tool_choice": "none"});
        let result = a2o(&req, "m");
        assert_eq!(result["tool_choice"], "none");
    }

    #[test]
    fn test_tool_choice_specific() {
        let req = json!({"messages": [], "tool_choice": {"name": "my_tool"}});
        let result = a2o(&req, "m");
        assert_eq!(result["tool_choice"]["type"], "function");
        assert_eq!(result["tool_choice"]["function"]["name"], "my_tool");
    }

    // --- openai_to_anthropic ---

    #[test]
    fn test_openai_text_response() {
        let resp = json!({
            "id": "chatcmpl-123",
            "model": "gpt-4",
            "choices": [{
                "message": {"role": "assistant", "content": "Hello!"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5}
        });
        let result = openai_to_anthropic(&resp, &empty_map()).unwrap();
        assert_eq!(result["type"], "message");
        assert_eq!(result["role"], "assistant");
        assert_eq!(result["model"], "gpt-4");
        assert_eq!(result["content"][0]["type"], "text");
        assert_eq!(result["content"][0]["text"], "Hello!");
        assert_eq!(result["stop_reason"], "end_turn");
        assert_eq!(result["usage"]["input_tokens"], 10);
        assert_eq!(result["usage"]["output_tokens"], 5);
    }

    #[test]
    fn test_openai_tool_call_response() {
        let resp = json!({
            "id": "chatcmpl-456",
            "model": "gpt-4",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_abc",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"city\":\"Tokyo\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 20, "completion_tokens": 15}
        });
        let result = openai_to_anthropic(&resp, &empty_map()).unwrap();
        assert_eq!(result["stop_reason"], "tool_use");
        assert_eq!(result["content"][0]["type"], "tool_use");
        assert_eq!(result["content"][0]["id"], "call_abc");
        assert_eq!(result["content"][0]["name"], "get_weather");
        assert_eq!(result["content"][0]["input"]["city"], "Tokyo");
    }

    #[test]
    fn test_stop_reason_mapping() {
        let make_resp = |reason: &str| {
            json!({
                "choices": [{"message": {"content": "x"}, "finish_reason": reason}],
                "usage": {}
            })
        };
        assert_eq!(
            openai_to_anthropic(&make_resp("stop"), &empty_map()).unwrap()["stop_reason"],
            "end_turn"
        );
        assert_eq!(
            openai_to_anthropic(&make_resp("length"), &empty_map()).unwrap()["stop_reason"],
            "max_tokens"
        );
        assert_eq!(
            openai_to_anthropic(&make_resp("tool_calls"), &empty_map()).unwrap()["stop_reason"],
            "tool_use"
        );
        assert_eq!(
            openai_to_anthropic(&make_resp("content_filter"), &empty_map()).unwrap()["stop_reason"],
            "end_turn"
        );
    }

    #[test]
    fn test_empty_openai_response() {
        let resp = json!({"choices": [], "usage": {}});
        let result = openai_to_anthropic(&resp, &empty_map()).unwrap();
        assert_eq!(result["type"], "message");
        assert!(result["content"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_tool_name_roundtrip() {
        let long_name = "mcp__claude_in_chrome__validate_and_render_mermaid_diagram_extra_long";
        let req = json!({
            "messages": [],
            "tools": [{
                "name": long_name,
                "description": "test",
                "input_schema": {}
            }]
        });
        let (body, map) = anthropic_to_openai(&req, "m").unwrap();
        let truncated = body["tools"][0]["function"]["name"].as_str().unwrap();
        assert!(truncated.len() <= 64);

        // 模拟 OpenAI 返回截断名
        let resp = json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "c1",
                        "type": "function",
                        "function": {"name": truncated, "arguments": "{}"}
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {}
        });
        let result = openai_to_anthropic(&resp, &map).unwrap();
        // 还原原始名字
        assert_eq!(result["content"][0]["name"], long_name);
    }
}
