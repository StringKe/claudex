use bytes::Bytes;
use futures::stream::{Stream, StreamExt};
use serde_json::{json, Value};
use std::pin::Pin;

/// Translates an OpenAI SSE stream to Anthropic SSE format.
///
/// OpenAI format:  `data: {"choices":[{"delta":{"content":"..."}}]}`
/// Anthropic format: multiple event types (message_start, content_block_start, content_block_delta, etc.)
pub fn translate_sse_stream<S>(
    input: S,
) -> Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
{
    let mut state = StreamState::new();

    let output = async_stream::stream! {
        // Send message_start
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

                    // Process complete SSE lines
                    while let Some(pos) = buffer.find("\n\n") {
                        let line = buffer[..pos].to_string();
                        buffer = buffer[pos + 2..].to_string();

                        if let Some(events) = state.process_openai_line(&line) {
                            for event in events {
                                yield Ok(Bytes::from(event));
                            }
                        }
                    }
                    // Also handle single newline delimited chunks
                    while let Some(pos) = buffer.find('\n') {
                        let line = buffer[..pos].to_string();
                        buffer = buffer[pos + 1..].to_string();

                        if line.is_empty() {
                            continue;
                        }

                        if let Some(events) = state.process_openai_line(&line) {
                            for event in events {
                                yield Ok(Bytes::from(event));
                            }
                        }
                    }
                }
                Err(e) => {
                    yield Err(e);
                    return;
                }
            }
        }

        // Send final events
        if state.block_started {
            let block_stop = format_sse("content_block_stop", &json!({
                "type": "content_block_stop",
                "index": state.block_index,
            }));
            yield Ok(Bytes::from(block_stop));
        }

        let msg_delta = format_sse("message_delta", &json!({
            "type": "message_delta",
            "delta": {"stop_reason": "end_turn", "stop_sequence": null},
            "usage": {"output_tokens": state.output_tokens}
        }));
        yield Ok(Bytes::from(msg_delta));

        yield Ok(Bytes::from(format_sse("message_stop", &json!({"type": "message_stop"}))));
    };

    Box::pin(output)
}

struct StreamState {
    block_index: usize,
    block_started: bool,
    output_tokens: u64,
    current_tool_call: Option<ToolCallState>,
}

struct ToolCallState {
    id: String,
    name: String,
    arguments_buffer: String,
}

impl StreamState {
    fn new() -> Self {
        Self {
            block_index: 0,
            block_started: false,
            output_tokens: 0,
            current_tool_call: None,
        }
    }

    fn process_openai_line(&mut self, line: &str) -> Option<Vec<String>> {
        let data = line.strip_prefix("data: ")?.trim();

        if data == "[DONE]" {
            return self.finalize_tool_call();
        }

        let parsed: Value = serde_json::from_str(data).ok()?;
        let choice = parsed.get("choices")?.as_array()?.first()?;
        let delta = choice.get("delta")?;

        let mut events = Vec::new();

        // Track usage
        if let Some(usage) = parsed.get("usage") {
            if let Some(tokens) = usage.get("completion_tokens").and_then(|t| t.as_u64()) {
                self.output_tokens = tokens;
            }
        }

        // Handle text content
        if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
            if !content.is_empty() {
                // Finalize any pending tool call first
                if let Some(tool_events) = self.finalize_tool_call() {
                    events.extend(tool_events);
                }

                if !self.block_started || self.current_tool_call.is_some() {
                    let block_start = format_sse(
                        "content_block_start",
                        &json!({
                            "type": "content_block_start",
                            "index": self.block_index,
                            "content_block": {"type": "text", "text": ""}
                        }),
                    );
                    events.push(block_start);
                    self.block_started = true;
                }

                let block_delta = format_sse(
                    "content_block_delta",
                    &json!({
                        "type": "content_block_delta",
                        "index": self.block_index,
                        "delta": {"type": "text_delta", "text": content}
                    }),
                );
                events.push(block_delta);
            }
        }

        // Handle tool calls
        if let Some(tool_calls) = delta.get("tool_calls").and_then(|tc| tc.as_array()) {
            for tc in tool_calls {
                let empty_func = json!({});
                let func = tc.get("function").unwrap_or(&empty_func);

                // New tool call starts
                if let Some(id) = tc.get("id").and_then(|id| id.as_str()) {
                    // Finalize previous blocks
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
                    if let Some(prev_events) = self.finalize_tool_call() {
                        events.extend(prev_events);
                    }

                    let name = func
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();

                    self.current_tool_call = Some(ToolCallState {
                        id: id.to_string(),
                        name: name.clone(),
                        arguments_buffer: String::new(),
                    });

                    events.push(format_sse(
                        "content_block_start",
                        &json!({
                            "type": "content_block_start",
                            "index": self.block_index,
                            "content_block": {
                                "type": "tool_use",
                                "id": id,
                                "name": name,
                                "input": {}
                            }
                        }),
                    ));
                    self.block_started = true;
                }

                // Accumulate arguments
                if let Some(args) = func.get("arguments").and_then(|a| a.as_str()) {
                    if let Some(ref mut tool_state) = self.current_tool_call {
                        tool_state.arguments_buffer.push_str(args);
                        events.push(format_sse(
                            "content_block_delta",
                            &json!({
                                "type": "content_block_delta",
                                "index": self.block_index,
                                "delta": {
                                    "type": "input_json_delta",
                                    "partial_json": args
                                }
                            }),
                        ));
                    }
                }
            }
        }

        // Handle finish_reason
        if let Some(finish) = choice.get("finish_reason").and_then(|f| f.as_str()) {
            if finish == "tool_calls" {
                if let Some(tool_events) = self.finalize_tool_call() {
                    events.extend(tool_events);
                }
            }
        }

        if events.is_empty() {
            None
        } else {
            Some(events)
        }
    }

    fn finalize_tool_call(&mut self) -> Option<Vec<String>> {
        let _tool_state = self.current_tool_call.take()?;
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

        Some(events)
    }
}

fn format_sse(event: &str, data: &Value) -> String {
    format!(
        "event: {event}\ndata: {}\n\n",
        serde_json::to_string(data).unwrap_or_default()
    )
}
