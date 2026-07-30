use bytes::Bytes;
use futures::stream::{Stream, StreamExt};
use serde_json::{json, Value};
use std::pin::Pin;

use crate::proxy::util::{format_sse, ToolNameMap};

/// Translates an OpenAI Responses API SSE stream to Anthropic SSE format.
///
/// Responses API events: response.created, response.output_text.delta, etc.
/// Anthropic events: message_start, content_block_start, content_block_delta, etc.
pub fn translate_responses_stream<S>(
    input: S,
    tool_name_map: ToolNameMap,
) -> Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
{
    let mut state = ResponsesStreamState::new(tool_name_map);

    let output = async_stream::stream! {
        // Send message_start immediately
        let msg_start = format_sse("message_start", &json!({
            "type": "message_start",
            "message": {
                "id": format!("msg_{}", uuid::Uuid::new_v4()),
                "type": "message",
                "role": "assistant",
                "model": "claudex-proxy",
                "content": [],
                "stop_reason": null,
                "stop_sequence": null,
                "usage": {"input_tokens": 0, "output_tokens": 0}
            }
        }));
        yield Ok(Bytes::from(msg_start));

        let mut stream = std::pin::pin!(input);
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    buffer.push_str(&String::from_utf8_lossy(&chunk));

                    // Process complete SSE events (separated by double newline or single newline)
                    while let Some(pos) = buffer.find('\n') {
                        let line = buffer[..pos].to_string();
                        buffer = buffer[pos + 1..].to_string();

                        if line.is_empty() {
                            continue;
                        }

                        for event in state.process_line(&line) {
                            yield Ok(Bytes::from(event));
                        }
                    }
                }
                Err(e) => {
                    yield Err(e);
                    return;
                }
            }
        }

        if state.error_emitted {
            return;
        }

        // Finalize: close any open block and send message_delta + message_stop
        if state.block_started {
            yield Ok(Bytes::from(format_sse("content_block_stop", &json!({
                "type": "content_block_stop",
                "index": state.block_index,
            }))));
        }

        let stop_reason = if state.has_tool_use { "tool_use" } else { &state.stop_reason };
        yield Ok(Bytes::from(format_sse("message_delta", &json!({
            "type": "message_delta",
            "delta": {"stop_reason": stop_reason, "stop_sequence": null},
            "usage": {
                "input_tokens": state.input_tokens,
                "cache_creation_input_tokens": state.cache_creation_input_tokens,
                "cache_read_input_tokens": state.cache_read_input_tokens,
                "output_tokens": state.output_tokens,
            }
        }))));
        yield Ok(Bytes::from(format_sse("message_stop", &json!({"type": "message_stop"}))));
    };

    Box::pin(output)
}

struct ResponsesStreamState {
    tool_name_map: ToolNameMap,
    block_index: usize,
    block_started: bool,
    has_tool_use: bool,
    stop_reason: String,
    input_tokens: u64,
    cache_creation_input_tokens: u64,
    cache_read_input_tokens: u64,
    output_tokens: u64,
    error_emitted: bool,
}

impl ResponsesStreamState {
    fn new(tool_name_map: ToolNameMap) -> Self {
        Self {
            tool_name_map,
            block_index: 0,
            block_started: false,
            has_tool_use: false,
            stop_reason: "end_turn".to_string(),
            input_tokens: 0,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            output_tokens: 0,
            error_emitted: false,
        }
    }

    fn process_line(&mut self, line: &str) -> Vec<String> {
        if self.error_emitted {
            return vec![];
        }

        // Responses API SSE format: "event: <type>\ndata: <json>" or just "data: <json>"
        // We may receive "event:" and "data:" lines separately
        if line.starts_with("event:") {
            // Event type line — we'll get the data in the next line
            return vec![];
        }

        let data = if let Some(stripped) = line.strip_prefix("data: ") {
            stripped
        } else if let Some(stripped) = line.strip_prefix("data:") {
            stripped
        } else {
            return vec![];
        };

        let json: Value = match serde_json::from_str(data) {
            Ok(v) => v,
            Err(_) => return vec![],
        };

        let event_type = json.get("type").and_then(|t| t.as_str()).unwrap_or("");
        match event_type {
            "response.output_text.delta" => {
                let delta = json.get("delta").and_then(|d| d.as_str()).unwrap_or("");
                if delta.is_empty() {
                    return vec![];
                }

                let mut events = Vec::new();

                // Start content block if not started
                if !self.block_started {
                    events.push(format_sse(
                        "content_block_start",
                        &json!({
                            "type": "content_block_start",
                            "index": self.block_index,
                            "content_block": {"type": "text", "text": ""},
                        }),
                    ));
                    self.block_started = true;
                }

                events.push(format_sse(
                    "content_block_delta",
                    &json!({
                        "type": "content_block_delta",
                        "index": self.block_index,
                        "delta": {"type": "text_delta", "text": delta},
                    }),
                ));

                events
            }
            "response.output_text.done" | "response.content_part.done" => {
                if self.block_started {
                    self.block_started = false;
                    let event = format_sse(
                        "content_block_stop",
                        &json!({
                            "type": "content_block_stop",
                            "index": self.block_index,
                        }),
                    );
                    self.block_index += 1;
                    return vec![event];
                }
                vec![]
            }
            "response.output_item.added" => {
                // Check if it's a function_call
                let empty = json!({});
                let item = json.get("item").unwrap_or(&empty);
                let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");

                if item_type == "function_call" {
                    self.has_tool_use = true;
                    let name = item
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("unknown");
                    let original_name = self
                        .tool_name_map
                        .get(name)
                        .cloned()
                        .unwrap_or(name.to_string());
                    let call_id = item
                        .get("call_id")
                        .and_then(|i| i.as_str())
                        .unwrap_or("call_0");

                    // Close any previous block
                    let mut events = Vec::new();
                    if self.block_started {
                        events.push(format_sse(
                            "content_block_stop",
                            &json!({
                                "type": "content_block_stop",
                                "index": self.block_index,
                            }),
                        ));
                        self.block_index += 1;
                        self.block_started = false;
                    }

                    events.push(format_sse(
                        "content_block_start",
                        &json!({
                            "type": "content_block_start",
                            "index": self.block_index,
                            "content_block": {
                                "type": "tool_use",
                                "id": call_id,
                                "name": original_name,
                                "input": {},
                            },
                        }),
                    ));
                    self.block_started = true;

                    return events;
                }
                vec![]
            }
            "response.function_call_arguments.delta" => {
                let delta = json.get("delta").and_then(|d| d.as_str()).unwrap_or("");
                if delta.is_empty() {
                    return vec![];
                }

                vec![format_sse(
                    "content_block_delta",
                    &json!({
                        "type": "content_block_delta",
                        "index": self.block_index,
                        "delta": {"type": "input_json_delta", "partial_json": delta},
                    }),
                )]
            }
            "response.function_call_arguments.done" => {
                if self.block_started {
                    self.block_started = false;
                    let event = format_sse(
                        "content_block_stop",
                        &json!({
                            "type": "content_block_stop",
                            "index": self.block_index,
                        }),
                    );
                    self.block_index += 1;
                    return vec![event];
                }
                vec![]
            }
            "response.completed" => {
                // Extract usage from the completed response
                if let Some(resp) = json.get("response") {
                    if let Some(usage) = resp.get("usage") {
                        let total_input_tokens = usage
                            .get("input_tokens")
                            .and_then(Value::as_u64)
                            .unwrap_or(0);
                        let cached_input_tokens = usage
                            .get("input_tokens_details")
                            .and_then(|details| details.get("cached_tokens"))
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                            .min(total_input_tokens);
                        self.input_tokens = total_input_tokens.saturating_sub(cached_input_tokens);
                        self.cache_creation_input_tokens = 0;
                        self.cache_read_input_tokens = cached_input_tokens;
                        self.output_tokens = usage
                            .get("output_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                    }
                    let status = resp
                        .get("status")
                        .and_then(|s| s.as_str())
                        .unwrap_or("completed");
                    if status == "incomplete" {
                        self.stop_reason = "max_tokens".to_string();
                    }
                }
                // Don't emit anything here — finalization happens in the outer stream
                vec![]
            }
            "response.failed" => {
                let error = json
                    .get("response")
                    .and_then(|response| response.get("error"));
                let code = error
                    .and_then(|error| error.get("code").or_else(|| error.get("type")))
                    .and_then(Value::as_str)
                    .unwrap_or("api_error");
                let error_type = if code.eq_ignore_ascii_case("context_length_exceeded") {
                    "invalid_request_error"
                } else {
                    "api_error"
                };
                let message = error
                    .and_then(|error| error.get("message"))
                    .and_then(Value::as_str)
                    .filter(|message| !message.is_empty())
                    .unwrap_or("Upstream response failed");
                self.error_emitted = true;
                vec![format_sse(
                    "error",
                    &json!({
                        "type": "error",
                        "error": {"type": error_type, "message": message},
                    }),
                )]
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    async fn translate(lines: &[&str]) -> String {
        let input = lines.join("\n\n") + "\n\n";
        let source = stream::iter(vec![Ok::<Bytes, reqwest::Error>(Bytes::from(input))]);
        let mut translated = translate_responses_stream(source, ToolNameMap::new());
        let mut result = String::new();
        while let Some(chunk) = translated.next().await {
            result.push_str(std::str::from_utf8(&chunk.unwrap()).unwrap());
        }
        result
    }

    fn accumulated_usage(output: &str) -> Value {
        let mut usage = serde_json::Map::new();
        for frame in output.split("\n\n") {
            let Some(data) = frame.lines().find_map(|line| line.strip_prefix("data: ")) else {
                continue;
            };
            let payload: Value = serde_json::from_str(data).unwrap();
            let event_usage = match payload.get("type").and_then(Value::as_str) {
                Some("message_start") => payload.pointer("/message/usage"),
                Some("message_delta") => payload.get("usage"),
                _ => None,
            };
            if let Some(event_usage) = event_usage.and_then(Value::as_object) {
                usage.extend(event_usage.clone());
            }
        }
        Value::Object(usage)
    }

    #[test]
    fn test_text_delta() {
        let mut state = ResponsesStreamState::new(ToolNameMap::new());
        let events = state.process_line(
            r#"data: {"type":"response.output_text.delta","delta":"Hello","output_index":0,"content_index":0}"#,
        );
        // Should get content_block_start + content_block_delta
        assert_eq!(events.len(), 2);
        assert!(events[0].contains("content_block_start"));
        assert!(events[1].contains("text_delta"));
        assert!(events[1].contains("Hello"));
    }

    #[test]
    fn test_function_call_flow() {
        let mut state = ResponsesStreamState::new(ToolNameMap::new());

        // function_call added
        let events = state.process_line(
            r#"data: {"type":"response.output_item.added","output_index":0,"item":{"type":"function_call","call_id":"call_1","name":"get_weather","arguments":"","status":"in_progress"}}"#,
        );
        assert_eq!(events.len(), 1);
        assert!(events[0].contains("tool_use"));
        assert!(events[0].contains("get_weather"));

        // argument delta
        let events = state.process_line(
            r#"data: {"type":"response.function_call_arguments.delta","delta":"{\"loc\""}"#,
        );
        assert_eq!(events.len(), 1);
        assert!(events[0].contains("input_json_delta"));

        // arguments done
        let events = state.process_line(
            r#"data: {"type":"response.function_call_arguments.done","name":"get_weather","arguments":"{\"location\":\"Paris\"}"}"#,
        );
        assert_eq!(events.len(), 1);
        assert!(events[0].contains("content_block_stop"));
        assert!(state.has_tool_use);
    }

    #[test]
    fn test_completed_extracts_usage() {
        let mut state = ResponsesStreamState::new(ToolNameMap::new());
        state.process_line(
            r#"data: {"type":"response.completed","response":{"status":"completed","usage":{"input_tokens":100,"output_tokens":50,"total_tokens":150}}}"#,
        );
        assert_eq!(state.input_tokens, 100);
        assert_eq!(state.cache_creation_input_tokens, 0);
        assert_eq!(state.cache_read_input_tokens, 0);
        assert_eq!(state.output_tokens, 50);
    }

    #[tokio::test]
    async fn test_completed_usage_is_accumulated_from_final_delta() {
        let output = translate(&[
            r#"data: {"type":"response.output_text.delta","delta":"Hello"}"#,
            r#"data: {"type":"response.completed","response":{"status":"completed","usage":{"input_tokens":12,"output_tokens":3}}}"#,
        ])
        .await;
        assert_eq!(
            accumulated_usage(&output),
            json!({
                "input_tokens": 12,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0,
                "output_tokens": 3,
            })
        );
    }

    #[tokio::test]
    async fn test_cached_input_tokens_are_not_double_counted() {
        let output = translate(&[
            r#"data: {"type":"response.output_text.delta","delta":"Hello"}"#,
            r#"data: {"type":"response.completed","response":{"status":"completed","usage":{"input_tokens":12,"input_tokens_details":{"cached_tokens":5},"output_tokens":3}}}"#,
        ])
        .await;
        assert_eq!(
            accumulated_usage(&output),
            json!({
                "input_tokens": 7,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 5,
                "output_tokens": 3,
            })
        );
    }

    #[tokio::test]
    async fn test_malformed_usage_fields_degrade_to_zero() {
        let output = translate(&[
            r#"data: {"type":"response.output_text.delta","delta":"Hello"}"#,
            r#"data: {"type":"response.completed","response":{"status":"completed","usage":{"input_tokens":"unknown","input_tokens_details":{"cached_tokens":5},"output_tokens":null}}}"#,
        ])
        .await;
        assert_eq!(
            accumulated_usage(&output),
            json!({
                "input_tokens": 0,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0,
                "output_tokens": 0,
            })
        );
    }

    #[tokio::test]
    async fn test_context_overflow_emits_only_invalid_request_error() {
        let output = translate(&[
            r#"data: {"type":"response.output_text.delta","delta":"partial"}"#,
            r#"data: {"type":"response.failed","response":{"status":"failed","error":{"code":"context_length_exceeded","message":"Context window exceeded"}}}"#,
        ])
        .await;
        assert_eq!(output.matches("event: error").count(), 1);
        assert!(output.contains("invalid_request_error"));
        assert!(output.contains("Context window exceeded"));
        assert!(!output.contains("content_block_stop"));
        assert!(!output.contains("message_delta"));
        assert!(!output.contains("message_stop"));
    }

    #[tokio::test]
    async fn test_unrelated_failure_remains_api_error_without_normal_stop() {
        let output = translate(&[
            r#"data: {"type":"response.failed","response":{"status":"failed","error":{"code":"server_error","message":"Upstream failed"}}}"#,
        ])
        .await;
        assert_eq!(output.matches("event: error").count(), 1);
        assert!(output.contains("api_error"));
        assert!(output.contains("Upstream failed"));
        assert!(!output.contains("message_delta"));
        assert!(!output.contains("message_stop"));
    }
}
