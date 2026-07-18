use super::errors::internal_error;
use super::response::{
    first_choice, required_object, required_usize, stream_usage, stream_usage_only_chunk,
};
use a4n_core::error::AppError;
use a4n_llm::provider::ChatChunk;
use serde_json::{Map, Value, json};
use std::collections::VecDeque;

pub(super) enum ParsedSseLine {
    Ignored,
    Chunk(Result<ChatChunk, AppError>),
    Done,
}

#[derive(Debug, Default)]
pub(super) struct SseDrainOutcome {
    pub(super) done: bool,
    pub(super) terminal_chunk_seen: bool,
    pub(super) parsed_chunks: usize,
}

impl SseDrainOutcome {
    fn observe(
        &mut self,
        parsed: ParsedSseLine,
        queue: &mut VecDeque<Result<ChatChunk, AppError>>,
    ) {
        match parsed {
            ParsedSseLine::Ignored => {}
            ParsedSseLine::Done => self.done = true,
            ParsedSseLine::Chunk(item) => {
                if let Ok(chunk) = &item {
                    self.parsed_chunks += 1;
                    self.terminal_chunk_seen |= chunk
                        .finish_reason
                        .as_deref()
                        .is_some_and(|reason| !reason.trim().is_empty());
                }
                queue.push_back(item);
            }
        }
    }
}

pub(super) fn parse_sse_data_line(line_bytes: &[u8]) -> ParsedSseLine {
    let line_bytes = line_bytes.strip_suffix(b"\r").unwrap_or(line_bytes);
    let line = match std::str::from_utf8(line_bytes) {
        Ok(line) => line.trim(),
        Err(e) => {
            return ParsedSseLine::Chunk(Err(AppError::Internal(anyhow::anyhow!(
                "解析流式响应 UTF-8 失败: {e}"
            ))));
        }
    };

    if line.is_empty() || line.starts_with(':') || !line.starts_with("data:") {
        return ParsedSseLine::Ignored;
    }

    let data = line.strip_prefix("data:").unwrap().trim();
    if data.is_empty() {
        return ParsedSseLine::Ignored;
    }
    if data == "[DONE]" {
        return ParsedSseLine::Done;
    }

    match serde_json::from_str::<Value>(data) {
        Ok(json) => {
            match stream_usage_only_chunk(&json) {
                Ok(Some(usage)) => {
                    return ParsedSseLine::Chunk(Ok(ChatChunk {
                        content: None,
                        reasoning_content: None,
                        tool_calls: None,
                        finish_reason: None,
                        index: 0,
                        usage: Some(usage),
                    }));
                }
                Ok(None) => {}
                Err(e) => return ParsedSseLine::Chunk(Err(e)),
            }

            let choice = match first_choice(&json, "choices", None) {
                Ok(choice) => choice,
                Err(e) => return ParsedSseLine::Chunk(Err(e)),
            };
            let usage = match stream_usage(&json) {
                Ok(usage) => usage,
                Err(e) => return ParsedSseLine::Chunk(Err(e)),
            };
            if let Err(e) = required_object(&choice["delta"], "choices[0].delta") {
                return ParsedSseLine::Chunk(Err(e));
            }
            let delta = &choice["delta"];
            let content = delta["content"].as_str().map(|s| s.to_string());
            let reasoning_content = delta["reasoning_content"].as_str().map(|s| s.to_string());
            let finish_reason = choice["finish_reason"].as_str().map(|s| s.to_string());
            let index = match required_usize(&choice["index"], "choices[0].index") {
                Ok(index) => match u32::try_from(index) {
                    Ok(index) => index,
                    Err(_) => {
                        return ParsedSseLine::Chunk(Err(internal_error(
                            "LLM 响应字段 choices[0].index 超出 u32 范围",
                        )));
                    }
                },
                Err(e) => return ParsedSseLine::Chunk(Err(e)),
            };

            ParsedSseLine::Chunk(Ok(ChatChunk {
                content,
                reasoning_content,
                tool_calls: None,
                finish_reason,
                index,
                usage,
            }))
        }
        Err(e) => {
            let preview: String = data.chars().take(200).collect();
            ParsedSseLine::Chunk(Err(internal_error(format!(
                "解析流式响应 JSON 失败: {e}; data={preview}"
            ))))
        }
    }
}

pub(super) fn drain_sse_lines(
    buffer: &mut Vec<u8>,
    queue: &mut VecDeque<Result<ChatChunk, AppError>>,
) -> SseDrainOutcome {
    let mut outcome = SseDrainOutcome::default();
    while let Some(pos) = buffer.iter().position(|b| *b == b'\n') {
        let mut line = buffer.drain(..=pos).collect::<Vec<_>>();
        if line.last() == Some(&b'\n') {
            line.pop();
        }
        outcome.observe(parse_sse_data_line(&line), queue);
        if outcome.done {
            buffer.clear();
            break;
        }
    }
    outcome
}

pub(super) fn drain_sse_tail(
    buffer: &mut Vec<u8>,
    queue: &mut VecDeque<Result<ChatChunk, AppError>>,
) -> SseDrainOutcome {
    let mut outcome = SseDrainOutcome::default();
    if !buffer.is_empty() {
        let tail = std::mem::take(buffer);
        outcome.observe(parse_sse_data_line(&tail), queue);
    }
    outcome
}

pub(super) fn enable_stream_usage(body: &mut Map<String, Value>) {
    match body.get_mut("stream_options") {
        Some(Value::Object(options)) => {
            options.insert("include_usage".to_string(), Value::Bool(true));
        }
        _ => {
            body.insert(
                "stream_options".to_string(),
                json!({ "include_usage": true }),
            );
        }
    }
}
