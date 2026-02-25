use bytes::Bytes;
use futures::stream::{Stream, StreamExt};
use serde_json::{json, Value};
use std::pin::Pin;

use super::responses::ToolNameMap;

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
            "usage": {"output_tokens": state.output_tokens}
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
    output_tokens: u64,
}

impl ResponsesStreamState {
    fn new(tool_name_map: ToolNameMap) -> Self {
        Self {
            tool_name_map,
            block_index: 0,
            block_started: false,
            has_tool_use: false,
            stop_reason: "end_turn".to_string(),
            output_tokens: 0,
        }
    }

    fn process_line(&mut self, line: &str) -> Vec<String> {
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
                self.stop_reason = "end_turn".to_string();
                vec![]
            }
            _ => vec![],
        }
    }
}

fn format_sse(event: &str, data: &Value) -> String {
    format!(
        "event: {event}\ndata: {}\n\n",
        serde_json::to_string(data).unwrap_or_default()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(state.output_tokens, 50);
    }
}
