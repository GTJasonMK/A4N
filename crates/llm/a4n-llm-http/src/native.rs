//! Native protocols share SSE framing; protocol modules own payload translation.

pub(super) mod gemini;
pub(super) mod responses;
#[cfg(test)]
mod tests;

use std::{collections::VecDeque, pin::Pin, time::Instant};

use a4n_error::AppError;
use a4n_llm_provider_contract::{ChatChunk, LlmProtocolKind};
use futures::{Stream, StreamExt};
use serde_json::Value;

use super::errors::stream_read_error;

const MAX_SSE_EVENT_BYTES: usize = 8 * 1024 * 1024;

pub(super) fn empty_chunk() -> ChatChunk {
    ChatChunk {
        content: None,
        reasoning_content: None,
        tool_calls: None,
        finish_reason: None,
        usage: None,
        index: 0,
    }
}

pub(super) fn required_string<'a>(value: &'a Value, field: &str) -> Result<&'a str, AppError> {
    value
        .as_str()
        .filter(|text| !text.is_empty())
        .ok_or_else(|| AppError::LlmError(format!("模型协议字段 {field} 必须是非空字符串")))
}

pub(super) fn function_definition(value: &Value) -> Result<&Value, AppError> {
    if value["type"] != "function" || !value["function"].is_object() {
        return Err(AppError::InvalidArgument(
            "模型工具必须是 function 定义".into(),
        ));
    }
    required_string(&value["function"]["name"], "function.name")?;
    Ok(&value["function"])
}

struct NativeStream<S> {
    source: Pin<Box<S>>,
    buffer: Vec<u8>,
    data: Vec<u8>,
    queue: VecDeque<Result<ChatChunk, AppError>>,
    eof: bool,
    terminal: bool,
    protocol: LlmProtocolKind,
    gemini: gemini::GeminiStream,
    received_bytes: usize,
    parsed_chunks: usize,
}

impl<S> NativeStream<S> {
    fn fail(&mut self, error: AppError) {
        self.eof = true;
        self.queue.push_back(Err(error));
    }

    fn event(&mut self) -> Result<(), AppError> {
        if self.data.is_empty() {
            return Ok(());
        }
        let bytes = std::mem::take(&mut self.data);
        if self.terminal && self.protocol == LlmProtocolKind::OpenAiResponses {
            return Err(AppError::LlmError("模型终态后收到额外 SSE 事件".into()));
        }
        let text = std::str::from_utf8(&bytes)
            .map_err(|error| AppError::LlmError(format!("模型 SSE UTF-8 无效: {error}")))?;
        let value = a4n_domain::json_contract::parse_strict_json_value(text)
            .map_err(|error| AppError::LlmError(format!("模型 SSE JSON 无效: {error}")))?;
        if !value["error"].is_null() {
            return Err(AppError::LlmError(format!(
                "模型 API 错误: {}",
                value["error"]
            )));
        }
        let chunks = match self.protocol {
            LlmProtocolKind::OpenAiResponses => responses::parse_event(&value)?,
            LlmProtocolKind::GeminiGenerateContent => self.gemini.parse_event(&value)?,
            _ => return Err(AppError::ConfigError("不支持的原生流协议".into())),
        };
        for chunk in chunks {
            self.terminal |= chunk.finish_reason.is_some();
            self.parsed_chunks += 1;
            self.queue.push_back(Ok(chunk));
        }
        if self.terminal && self.protocol == LlmProtocolKind::OpenAiResponses {
            self.eof = true;
        }
        Ok(())
    }

    fn line(&mut self, line: &[u8]) -> Result<(), AppError> {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() {
            return self.event();
        }
        if let Some(data) = line.strip_prefix(b"data:") {
            if self.data.len().saturating_add(data.len()) >= MAX_SSE_EVENT_BYTES {
                return Err(AppError::LlmError("模型 SSE 事件超过 8 MiB".into()));
            }
            if !self.data.is_empty() {
                self.data.push(b'\n');
            }
            self.data
                .extend_from_slice(data.strip_prefix(b" ").unwrap_or(data));
        }
        Ok(())
    }

    fn drain(&mut self) -> Result<(), AppError> {
        if let Some(index) = self.buffer.iter().rposition(|byte| *byte == b'\n') {
            let complete: Vec<_> = self.buffer.drain(..=index).collect();
            for line in complete.split_inclusive(|byte| *byte == b'\n') {
                self.line(&line[..line.len() - 1])?;
                if self.eof {
                    break;
                }
            }
        }
        if self.buffer.len() > MAX_SSE_EVENT_BYTES {
            return Err(AppError::LlmError("模型 SSE 行超过 8 MiB".into()));
        }
        Ok(())
    }
}

pub(super) fn chunk_stream<S, B>(
    source: S,
    protocol: LlmProtocolKind,
    timeout_secs: u64,
    model: String,
    started: Instant,
) -> impl Stream<Item = Result<ChatChunk, AppError>> + Send
where
    S: Stream<Item = Result<B, reqwest::Error>> + Send + 'static,
    B: AsRef<[u8]> + Send + 'static,
{
    let state = NativeStream {
        source: Box::pin(source),
        buffer: Vec::new(),
        data: Vec::new(),
        queue: VecDeque::new(),
        eof: false,
        terminal: false,
        protocol,
        gemini: gemini::GeminiStream::default(),
        received_bytes: 0,
        parsed_chunks: 0,
    };
    futures::stream::unfold(state, move |mut state| {
        let model = model.clone();
        async move {
            loop {
                if let Some(item) = state.queue.pop_front() {
                    return Some((item, state));
                }
                if state.eof {
                    return None;
                }
                match state.source.next().await {
                    Some(Ok(bytes)) => {
                        state.received_bytes += bytes.as_ref().len();
                        state.buffer.extend_from_slice(bytes.as_ref());
                        if let Err(error) = state.drain() {
                            state.fail(error);
                        }
                    }
                    Some(Err(error)) => {
                        state.fail(stream_read_error(
                            &error,
                            timeout_secs,
                            &model,
                            state.received_bytes,
                            state.parsed_chunks,
                            started.elapsed(),
                        ));
                    }
                    None => {
                        state.eof = true;
                        let tail = std::mem::take(&mut state.buffer);
                        if let Err(error) = state.line(&tail).and_then(|_| state.event()) {
                            state.fail(error);
                        } else if !state.terminal {
                            state.fail(AppError::LlmError("模型流结束但缺少协议终态".into()));
                        }
                    }
                }
            }
        }
    })
}
