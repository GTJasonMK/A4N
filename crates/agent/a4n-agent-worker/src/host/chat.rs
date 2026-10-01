use a4n_agent_runtime::AgentChatStream;
use a4n_error::AppError;
use a4n_llm_provider_contract::ChatChunk;
use futures::stream;
use reqwest::Response;
use serde_json::Value;

struct ChatSseState {
    response: Response,
    buffer: String,
    finished: bool,
}

pub(super) fn chat_sse_stream(response: Response) -> AgentChatStream {
    Box::pin(stream::unfold(
        ChatSseState {
            response,
            buffer: String::new(),
            finished: false,
        },
        |mut state| async move {
            loop {
                if let Some(item) = state.next_event(false) {
                    return Some((item, state));
                }
                if state.finished {
                    return state.next_event(true).map(|item| (item, state));
                }
                match state.response.chunk().await {
                    Ok(Some(bytes)) => {
                        state.buffer.push_str(&String::from_utf8_lossy(&bytes));
                    }
                    Ok(None) => state.finished = true,
                    Err(error) => {
                        state.finished = true;
                        return Some((
                            Err(AppError::LlmError(format!(
                                "读取 Control Agent LLM 流失败: {error}"
                            ))),
                            state,
                        ));
                    }
                }
            }
        },
    ))
}

impl ChatSseState {
    fn next_event(&mut self, flush: bool) -> Option<Result<ChatChunk, AppError>> {
        let end = self
            .buffer
            .find("\n\n")
            .or_else(|| flush.then_some(self.buffer.len()))?;
        if end == 0 && !flush {
            return None;
        }
        let raw = self.buffer[..end].to_string();
        let consumed = end.saturating_add(2).min(self.buffer.len());
        self.buffer.drain(..consumed);
        let mut event_name = "message";
        let mut data = Vec::new();
        for line in raw.lines() {
            let line = line.trim_end_matches('\r');
            if let Some(value) = line.strip_prefix("event:") {
                event_name = value.trim();
            } else if let Some(value) = line.strip_prefix("data:") {
                data.push(value.trim_start().to_string());
            }
        }
        if data.is_empty() {
            return None;
        }
        let data = data.join("\n");
        if event_name == "error" {
            return Some(Err(AppError::LlmError(
                serde_json::from_str::<Value>(&data)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("message")
                            .or_else(|| value.get("error"))
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
                    .unwrap_or(data),
            )));
        }
        Some(
            serde_json::from_str(&data).map_err(|error| {
                AppError::LlmError(format!("Control Agent LLM 流数据无效: {error}"))
            }),
        )
    }
}
