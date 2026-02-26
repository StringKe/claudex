use std::collections::HashMap;

use anyhow::Result;
use serde_json::{json, Value};

use crate::proxy::util::{truncate_tool_name, ToolNameMap};

/// Convert Anthropic Messages API request → OpenAI Responses API request
pub fn anthropic_to_responses(
    anthropic: &Value,
    default_model: &str,
) -> Result<(Value, ToolNameMap)> {
    let mut tool_name_map: ToolNameMap = HashMap::new();
    let mut input: Vec<Value> = Vec::new();

    // Convert messages → input items
    if let Some(msgs) = anthropic.get("messages").and_then(|m| m.as_array()) {
        for msg in msgs {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
            let content = msg.get("content");

            match role {
                "user" => {
                    // Check if this is a tool_result message
                    let has_tool_result = content.and_then(|c| c.as_array()).is_some_and(|arr| {
                        arr.iter()
                            .any(|b| b.get("type").and_then(|t| t.as_str()) == Some("tool_result"))
                    });

                    if has_tool_result {
                        if let Some(blocks) = content.and_then(|c| c.as_array()) {
                            for block in blocks {
                                if block.get("type").and_then(|t| t.as_str()) == Some("tool_result")
                                {
                                    let call_id = block
                                        .get("tool_use_id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("call_0");
                                    let output = extract_tool_result_content(block);
                                    input.push(json!({
                                        "type": "function_call_output",
                                        "call_id": call_id,
                                        "output": output,
                                    }));
                                }
                            }
                        }
                    } else {
                        let parts = convert_user_content(content);
                        input.push(json!({
                            "role": "user",
                            "type": "message",
                            "content": parts,
                        }));
                    }
                }
                "assistant" => {
                    // Assistant messages may contain text and tool_use blocks
                    let content_array = match content {
                        Some(Value::Array(arr)) => arr.clone(),
                        Some(Value::String(s)) => vec![json!({"type": "text", "text": s})],
                        _ => vec![],
                    };

                    let mut text_parts = Vec::new();
                    for block in &content_array {
                        let block_type = block.get("type").and_then(|t| t.as_str());
                        match block_type {
                            Some("text") => {
                                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                    text_parts.push(json!({
                                        "type": "output_text",
                                        "text": text,
                                        "annotations": [],
                                    }));
                                }
                            }
                            Some("tool_use") => {
                                // tool_use → function_call (separate input item)
                                // First, flush text parts as a message
                                if !text_parts.is_empty() {
                                    input.push(json!({
                                        "type": "message",
                                        "role": "assistant",
                                        "status": "completed",
                                        "content": text_parts,
                                    }));
                                    text_parts = Vec::new();
                                }

                                let name = block
                                    .get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("unknown");
                                let id =
                                    block.get("id").and_then(|i| i.as_str()).unwrap_or("call_0");
                                let truncated = truncate_tool_name(name);
                                if truncated != name {
                                    tool_name_map.insert(truncated.clone(), name.to_string());
                                }
                                let arguments = block
                                    .get("input")
                                    .map(|v| serde_json::to_string(v).unwrap_or_default())
                                    .unwrap_or_else(|| "{}".to_string());

                                input.push(json!({
                                    "type": "function_call",
                                    "call_id": id,
                                    "name": truncated,
                                    "arguments": arguments,
                                    "status": "completed",
                                }));
                            }
                            _ => {}
                        }
                    }
                    // Flush remaining text parts
                    if !text_parts.is_empty() {
                        input.push(json!({
                            "type": "message",
                            "role": "assistant",
                            "status": "completed",
                            "content": text_parts,
                        }));
                    }
                }
                _ => {
                    // Generic user message fallback
                    let text = match content {
                        Some(Value::String(s)) => s.clone(),
                        _ => String::new(),
                    };
                    if !text.is_empty() {
                        input.push(json!({
                            "role": "user",
                            "type": "message",
                            "content": [{"type": "input_text", "text": text}],
                        }));
                    }
                }
            }
        }
    }

    // System prompt → instructions
    let instructions = anthropic
        .get("system")
        .map(|s| match s {
            Value::String(s) => s.clone(),
            Value::Array(parts) => parts
                .iter()
                .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("\n"),
            _ => String::new(),
        })
        .unwrap_or_default();

    // Model
    let model = anthropic
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or(default_model);

    // Build request body
    let mut body = json!({
        "model": model,
        "input": input,
        "stream": anthropic.get("stream").and_then(|s| s.as_bool()).unwrap_or(false),
        "store": false,
    });

    if !instructions.is_empty() {
        body["instructions"] = json!(instructions);
    }

    // 注意：ChatGPT 后端不支持 max_output_tokens，跳过该参数

    // temperature, top_p
    if let Some(temp) = anthropic.get("temperature") {
        body["temperature"] = temp.clone();
    }
    if let Some(top_p) = anthropic.get("top_p") {
        body["top_p"] = top_p.clone();
    }

    // Tools
    if let Some(tools) = anthropic.get("tools").and_then(|t| t.as_array()) {
        let resp_tools: Vec<Value> = tools
            .iter()
            .map(|tool| {
                let name = tool.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
                let truncated = truncate_tool_name(name);
                if truncated != name {
                    tool_name_map.insert(truncated.clone(), name.to_string());
                }
                json!({
                    "type": "function",
                    "name": truncated,
                    "description": tool.get("description").cloned().unwrap_or(json!("")),
                    "parameters": tool.get("input_schema").cloned().unwrap_or(json!({"type": "object"})),
                })
            })
            .collect();
        body["tools"] = json!(resp_tools);
    }

    // tool_choice
    if let Some(tc) = anthropic.get("tool_choice") {
        let tc_type = tc.get("type").and_then(|t| t.as_str()).unwrap_or("auto");
        body["tool_choice"] = match tc_type {
            "auto" => json!("auto"),
            "any" => json!("required"),
            "none" => json!("none"),
            "tool" => {
                let name = tc.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let truncated = truncate_tool_name(name);
                json!({"type": "function", "name": truncated})
            }
            _ => json!("auto"),
        };
    }

    Ok((body, tool_name_map))
}

/// Convert OpenAI Responses API response → Anthropic Messages API response
pub fn responses_to_anthropic(resp: &Value, tool_name_map: &ToolNameMap) -> Result<Value> {
    let mut content = Vec::new();
    let mut has_tool_use = false;

    if let Some(output) = resp.get("output").and_then(|o| o.as_array()) {
        for item in output {
            let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");

            match item_type {
                "message" => {
                    if let Some(parts) = item.get("content").and_then(|c| c.as_array()) {
                        for part in parts {
                            let part_type = part.get("type").and_then(|t| t.as_str()).unwrap_or("");
                            if part_type == "output_text" {
                                if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                    content.push(json!({
                                        "type": "text",
                                        "text": text,
                                    }));
                                }
                            }
                        }
                    }
                }
                "function_call" => {
                    has_tool_use = true;
                    let name = item
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("unknown");
                    let original_name =
                        tool_name_map.get(name).cloned().unwrap_or(name.to_string());
                    let call_id = item
                        .get("call_id")
                        .and_then(|i| i.as_str())
                        .unwrap_or("call_0");
                    let arguments = item
                        .get("arguments")
                        .and_then(|a| a.as_str())
                        .unwrap_or("{}");
                    let input: Value =
                        serde_json::from_str(arguments).unwrap_or_else(|_| json!({}));

                    content.push(json!({
                        "type": "tool_use",
                        "id": call_id,
                        "name": original_name,
                        "input": input,
                    }));
                }
                _ => {}
            }
        }
    }

    // stop_reason
    let status = resp
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("completed");
    let stop_reason = if has_tool_use {
        "tool_use"
    } else {
        match status {
            "completed" => "end_turn",
            "incomplete" => "max_tokens",
            _ => "end_turn",
        }
    };

    // usage
    let usage = resp.get("usage").cloned().unwrap_or(json!({}));
    let anthropic_usage = json!({
        "input_tokens": usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        "output_tokens": usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
    });

    let model = resp
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("unknown");
    let id = resp.get("id").and_then(|i| i.as_str()).unwrap_or("resp_0");

    Ok(json!({
        "id": id,
        "type": "message",
        "role": "assistant",
        "model": model,
        "content": content,
        "stop_reason": stop_reason,
        "stop_sequence": null,
        "usage": anthropic_usage,
    }))
}

fn convert_user_content(content: Option<&Value>) -> Vec<Value> {
    match content {
        Some(Value::String(s)) => vec![json!({"type": "input_text", "text": s})],
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|p| {
                let block_type = p.get("type").and_then(|t| t.as_str());
                match block_type {
                    Some("text") => {
                        let text = p.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        Some(json!({"type": "input_text", "text": text}))
                    }
                    Some("image") => {
                        // Anthropic base64 image → OpenAI image_url
                        let source = p.get("source");
                        let media_type = source
                            .and_then(|s| s.get("media_type"))
                            .and_then(|m| m.as_str())
                            .unwrap_or("image/png");
                        let data = source
                            .and_then(|s| s.get("data"))
                            .and_then(|d| d.as_str())
                            .unwrap_or("");
                        Some(json!({
                            "type": "input_image",
                            "image_url": format!("data:{media_type};base64,{data}"),
                        }))
                    }
                    Some("tool_result") => {
                        // tool_result at user level → function_call_output
                        // This shouldn't normally appear here but handle it
                        None
                    }
                    _ => None,
                }
            })
            .collect(),
        _ => vec![],
    }
}

fn extract_tool_result_content(block: &Value) -> String {
    let content = block.get("content");
    match content {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_user_message() {
        let anthropic = json!({
            "model": "gpt-4o",
            "system": "You are helpful.",
            "messages": [
                {"role": "user", "content": "Hello"}
            ],
            "max_tokens": 1024,
            "stream": false,
        });
        let (body, map) = anthropic_to_responses(&anthropic, "gpt-4o").unwrap();
        assert_eq!(body["model"], "gpt-4o");
        assert_eq!(body["instructions"], "You are helpful.");
        assert!(body.get("max_output_tokens").is_none());
        assert_eq!(body["store"], false);
        assert_eq!(body["stream"], false);
        let input = body["input"].as_array().unwrap();
        assert_eq!(input.len(), 1);
        assert_eq!(input[0]["role"], "user");
        assert_eq!(input[0]["content"][0]["type"], "input_text");
        assert_eq!(input[0]["content"][0]["text"], "Hello");
        assert!(map.is_empty());
    }

    #[test]
    fn test_tool_use_roundtrip() {
        let anthropic = json!({
            "model": "gpt-4o",
            "messages": [
                {"role": "user", "content": "What's the weather?"},
                {"role": "assistant", "content": [
                    {"type": "tool_use", "id": "call_1", "name": "get_weather", "input": {"location": "Paris"}}
                ]},
                {"role": "user", "content": [
                    {"type": "tool_result", "tool_use_id": "call_1", "content": "Sunny, 25°C"}
                ]},
            ],
            "tools": [
                {"name": "get_weather", "description": "Get weather", "input_schema": {"type": "object", "properties": {"location": {"type": "string"}}}}
            ],
            "max_tokens": 1024,
        });

        let (body, _map) = anthropic_to_responses(&anthropic, "gpt-4o").unwrap();
        let input = body["input"].as_array().unwrap();
        // user message + function_call + function_call_output
        assert_eq!(input.len(), 3);
        assert_eq!(input[0]["role"], "user");
        assert_eq!(input[1]["type"], "function_call");
        assert_eq!(input[1]["name"], "get_weather");
        assert_eq!(input[2]["type"], "function_call_output");
        assert_eq!(input[2]["call_id"], "call_1");

        // Tools
        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["name"], "get_weather");
    }

    #[test]
    fn test_responses_to_anthropic_text() {
        let resp = json!({
            "id": "resp_123",
            "model": "gpt-4o",
            "status": "completed",
            "output": [
                {
                    "type": "message",
                    "role": "assistant",
                    "status": "completed",
                    "content": [
                        {"type": "output_text", "text": "Hello!", "annotations": []}
                    ]
                }
            ],
            "usage": {"input_tokens": 10, "output_tokens": 5, "total_tokens": 15},
        });
        let result = responses_to_anthropic(&resp, &HashMap::new()).unwrap();
        assert_eq!(result["stop_reason"], "end_turn");
        assert_eq!(result["content"][0]["type"], "text");
        assert_eq!(result["content"][0]["text"], "Hello!");
        assert_eq!(result["usage"]["input_tokens"], 10);
    }

    #[test]
    fn test_responses_to_anthropic_tool_call() {
        let resp = json!({
            "id": "resp_456",
            "model": "gpt-4o",
            "status": "completed",
            "output": [
                {
                    "type": "function_call",
                    "call_id": "call_abc",
                    "name": "get_weather",
                    "arguments": "{\"location\":\"Paris\"}",
                    "status": "completed",
                }
            ],
            "usage": {"input_tokens": 20, "output_tokens": 10},
        });
        let result = responses_to_anthropic(&resp, &HashMap::new()).unwrap();
        assert_eq!(result["stop_reason"], "tool_use");
        assert_eq!(result["content"][0]["type"], "tool_use");
        assert_eq!(result["content"][0]["id"], "call_abc");
        assert_eq!(result["content"][0]["name"], "get_weather");
        assert_eq!(result["content"][0]["input"]["location"], "Paris");
    }

    #[test]
    fn test_tool_choice_mapping() {
        let test_cases = vec![
            (json!({"type": "auto"}), json!("auto")),
            (json!({"type": "any"}), json!("required")),
            (json!({"type": "none"}), json!("none")),
            (
                json!({"type": "tool", "name": "fn1"}),
                json!({"type": "function", "name": "fn1"}),
            ),
        ];
        for (anthropic_tc, expected) in test_cases {
            let anthropic = json!({
                "model": "gpt-4o",
                "messages": [{"role": "user", "content": "test"}],
                "tool_choice": anthropic_tc,
                "max_tokens": 100,
            });
            let (body, _) = anthropic_to_responses(&anthropic, "gpt-4o").unwrap();
            assert_eq!(body["tool_choice"], expected);
        }
    }

    #[test]
    fn test_system_prompt_array() {
        let anthropic = json!({
            "model": "gpt-4o",
            "system": [
                {"type": "text", "text": "Part 1."},
                {"type": "text", "text": "Part 2."},
            ],
            "messages": [{"role": "user", "content": "Hi"}],
            "max_tokens": 100,
        });
        let (body, _) = anthropic_to_responses(&anthropic, "gpt-4o").unwrap();
        assert_eq!(body["instructions"], "Part 1.\nPart 2.");
    }
}
