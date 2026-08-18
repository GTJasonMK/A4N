use std::{collections::HashMap, pin::Pin, time::Instant};

use a4n_core::error::AppError;
use a4n_llm::provider::{
    ChatChunk, ChatMessage, ChatRequest, LlmProvider, MessageRole, ModelCapabilities, ToolCall,
    UsageInfo,
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::{Map, Value, json};

use super::errors::{
    build_http_client, normalized_connect_timeout_secs, normalized_timeout_secs, request_error,
    response_text,
};
use super::response::llm_api_status_error;

pub struct AnthropicLlmProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
    model: String,
    timeout_secs: u64,
    connect_timeout_secs: u64,
    extra_body: Option<Map<String, Value>>,
    caps: ModelCapabilities,
}

impl AnthropicLlmProvider {
    pub fn new(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
        timeout_secs: u64,
        extra_body: Option<Value>,
        caps: ModelCapabilities,
    ) -> Result<Self, AppError> {
        let timeout_secs = normalized_timeout_secs(timeout_secs);
        let connect_timeout_secs = normalized_connect_timeout_secs(timeout_secs);
        Ok(Self {
            client: build_http_client(timeout_secs, connect_timeout_secs),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
            model: model.into(),
            timeout_secs,
            connect_timeout_secs,
            extra_body: a4n_llm::provider::normalize_extra_body(extra_body)?,
            caps,
        })
    }

    fn request_body(&self, request: &ChatRequest) -> Result<Value, AppError> {
        let mut body = self.extra_body.clone().unwrap_or_default();
        body.insert("model".into(), json!(self.model));
        body.insert(
            "max_tokens".into(),
            json!(request.max_tokens.unwrap_or(self.caps.max_output_tokens)),
        );
        body.insert("stream".into(), Value::Bool(true));
        if let Some(temperature) = request.temperature {
            body.insert("temperature".into(), json!(temperature));
        }
        if let Some(stop) = &request.stop_sequences {
            body.insert("stop_sequences".into(), json!(stop));
        }

        let mut system = request
            .messages
            .iter()
            .filter(|message| matches!(message.role, MessageRole::System))
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        if request.response_format.is_some() {
            if !system.is_empty() {
                system.push_str("\n\n");
            }
            system.push_str("Return only one valid JSON value. Do not wrap it in Markdown fences or add explanatory text.");
        }
        if !system.is_empty() {
            let mut block = json!({"type": "text", "text": system});
            if request.cache_system_prefix {
                block["cache_control"] = json!({"type": "ephemeral"});
            }
            body.insert("system".into(), Value::Array(vec![block]));
        }

        body.insert("messages".into(), anthropic_messages(&request.messages)?);
        if let Some(tools) = &request.tools {
            body.insert("tools".into(), anthropic_tools(tools)?);
        }
        if let Some(choice) = request.tool_choice.as_deref() {
            body.insert("tool_choice".into(), anthropic_tool_choice(choice));
        }
        Ok(Value::Object(body))
    }

    async fn execute(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        let started = Instant::now();
        let mut builder = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("anthropic-version", "2023-06-01")
            .header("accept", "text/event-stream")
            .json(&self.request_body(&request)?);
        if let Some(key) = self.api_key.as_deref().filter(|key| !key.trim().is_empty()) {
            builder = builder.header("x-api-key", key);
        }
        let response = builder.send().await.map_err(|error| {
            request_error(
                "Anthropic Messages 流式",
                error,
                self.timeout_secs,
                self.connect_timeout_secs,
                started.elapsed(),
            )
        })?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response_text(response, "Anthropic Messages", self.timeout_secs).await?;
            return Err(llm_api_status_error(status, &body));
        }
        Ok(Box::pin(anthropic_chunk_stream(response.bytes_stream())))
    }
}

#[async_trait]
impl LlmProvider for AnthropicLlmProvider {
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        self.execute(request).await
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn capabilities(&self) -> ModelCapabilities {
        self.caps.clone()
    }

    fn count_tokens(&self, text: &str) -> usize {
        approximate_tokens(text)
    }
}

fn approximate_tokens(text: &str) -> usize {
    let chinese_chars = text.chars().filter(|c| u32::from(*c) > 0x2E80).count();
    let other_words = text.split_whitespace().count();
    ((chinese_chars as f32 * 0.5) + (other_words as f32 * 1.3)).ceil() as usize
}

fn anthropic_messages(messages: &[ChatMessage]) -> Result<Value, AppError> {
    let mut output = Vec::new();
    for message in messages {
        if matches!(message.role, MessageRole::System) {
            continue;
        }
        match message.role {
            MessageRole::User => output.push(json!({"role": "user", "content": message.content})),
            MessageRole::Assistant => {
                let mut blocks = Vec::new();
                if !message.content.is_empty() {
                    blocks.push(json!({"type": "text", "text": message.content}));
                }
                if let Some(calls) = &message.tool_calls {
                    blocks.extend(calls.iter().map(|call| json!({
                        "type": "tool_use", "id": call.id, "name": call.name, "input": call.arguments
                    })));
                }
                output.push(json!({"role": "assistant", "content": blocks}));
            }
            MessageRole::Tool => {
                let id = message.tool_call_id.as_deref().ok_or_else(|| {
                    AppError::InvalidArgument("Anthropic tool消息缺少 tool_call_id".to_string())
                })?;
                output.push(json!({"role": "user", "content": [{
                    "type": "tool_result", "tool_use_id": id, "content": message.content
                }]}));
            }
            MessageRole::System => unreachable!(),
        }
    }
    Ok(Value::Array(output))
}

fn anthropic_tools(tools: &[Value]) -> Result<Value, AppError> {
    tools
        .iter()
        .map(|tool| {
            let function = tool.get("function").unwrap_or(tool);
            let name = function.get("name").and_then(Value::as_str).ok_or_else(|| {
                AppError::InvalidArgument("LLM tool缺少 function.name".to_string())
            })?;
            Ok(json!({
                "name": name,
                "description": function.get("description").cloned().unwrap_or(Value::Null),
                "input_schema": function.get("parameters").cloned().unwrap_or_else(|| json!({"type": "object"}))
            }))
        })
        .collect::<Result<Vec<_>, AppError>>()
        .map(Value::Array)
}

fn anthropic_tool_choice(choice: &str) -> Value {
    match choice {
        "auto" => json!({"type": "auto"}),
        "required" => json!({"type": "any"}),
        "none" => json!({"type": "none"}),
        name => json!({"type": "tool", "name": name}),
    }
}

#[derive(Default)]
struct AnthropicStreamState {
    bytes: Vec<u8>,
    queued: std::collections::VecDeque<Result<ChatChunk, AppError>>,
    tools: HashMap<u32, PendingTool>,
    usage: UsageState,
    index: u32,
    finished: bool,
}

struct PendingTool {
    id: String,
    name: String,
    input_json: String,
}

#[derive(Default)]
struct UsageState {
    input: usize,
    output: usize,
    cache_hit: Option<usize>,
    cache_miss: Option<usize>,
}

fn anthropic_chunk_stream<S, B>(stream: S) -> impl Stream<Item = Result<ChatChunk, AppError>> + Send
where
    S: Stream<Item = Result<B, reqwest::Error>> + Send + 'static,
    B: AsRef<[u8]> + Send + 'static,
{
    futures::stream::unfold(
        (Box::pin(stream), AnthropicStreamState::default()),
        |(mut stream, mut state)| async move {
            loop {
                if let Some(item) = state.queued.pop_front() {
                    return Some((item, (stream, state)));
                }
                if state.finished {
                    return None;
                }
                match stream.as_mut().next().await {
                    Some(Ok(bytes)) => {
                        state.bytes.extend_from_slice(bytes.as_ref());
                        drain_anthropic_lines(&mut state);
                    }
                    Some(Err(error)) => {
                        state.finished = true;
                        return Some((
                            Err(AppError::LlmError(format!("Anthropic流读取失败: {error}"))),
                            (stream, state),
                        ));
                    }
                    None => {
                        if !state.bytes.is_empty() {
                            let tail = std::mem::take(&mut state.bytes);
                            parse_anthropic_line(&tail, &mut state);
                        }
                        state.finished = true;
                    }
                }
            }
        },
    )
}

fn drain_anthropic_lines(state: &mut AnthropicStreamState) {
    while let Some(pos) = state.bytes.iter().position(|byte| *byte == b'\n') {
        let mut line = state.bytes.drain(..=pos).collect::<Vec<_>>();
        line.pop();
        parse_anthropic_line(&line, state);
        if state.finished {
            state.bytes.clear();
            break;
        }
    }
}

fn parse_anthropic_line(line: &[u8], state: &mut AnthropicStreamState) {
    let Ok(line) = std::str::from_utf8(line) else {
        state.queued.push_back(Err(AppError::LlmError(
            "Anthropic SSE不是合法UTF-8".to_string(),
        )));
        return;
    };
    let Some(data) = line.trim().strip_prefix("data:").map(str::trim) else {
        return;
    };
    if data.is_empty() {
        return;
    }
    let event: Value = match serde_json::from_str(data) {
        Ok(event) => event,
        Err(error) => {
            state.queued.push_back(Err(AppError::LlmError(format!(
                "Anthropic SSE JSON解析失败: {error}"
            ))));
            return;
        }
    };
    if let Err(error) = project_anthropic_event(&event, state) {
        state.queued.push_back(Err(error));
    }
}

fn project_anthropic_event(
    event: &Value,
    state: &mut AnthropicStreamState,
) -> Result<(), AppError> {
    match event.get("type").and_then(Value::as_str).unwrap_or("") {
        "message_start" => {
            let usage = &event["message"]["usage"];
            state.usage.input = usage["input_tokens"].as_u64().unwrap_or(0) as usize;
            state.usage.cache_hit = usage["cache_read_input_tokens"]
                .as_u64()
                .map(|v| v as usize);
            state.usage.cache_miss = usage["cache_creation_input_tokens"]
                .as_u64()
                .map(|v| v as usize);
        }
        "content_block_start" => {
            let index = event["index"].as_u64().unwrap_or(0) as u32;
            let block = &event["content_block"];
            if block["type"] == "tool_use" {
                state.tools.insert(
                    index,
                    PendingTool {
                        id: required_string(block, "id")?,
                        name: required_string(block, "name")?,
                        input_json: String::new(),
                    },
                );
            } else if let Some(text) = block["text"].as_str().filter(|text| !text.is_empty()) {
                push_chunk(state, Some(text.to_string()), None, None, None);
            }
        }
        "content_block_delta" => {
            let index = event["index"].as_u64().unwrap_or(0) as u32;
            let delta = &event["delta"];
            match delta["type"].as_str().unwrap_or("") {
                "text_delta" => push_chunk(
                    state,
                    delta["text"].as_str().map(str::to_string),
                    None,
                    None,
                    None,
                ),
                "thinking_delta" => push_chunk(
                    state,
                    None,
                    delta["thinking"].as_str().map(str::to_string),
                    None,
                    None,
                ),
                "input_json_delta" => {
                    if let Some(tool) = state.tools.get_mut(&index) {
                        tool.input_json
                            .push_str(delta["partial_json"].as_str().unwrap_or(""));
                    }
                }
                _ => {}
            }
        }
        "content_block_stop" => {
            let index = event["index"].as_u64().unwrap_or(0) as u32;
            if let Some(tool) = state.tools.remove(&index) {
                let arguments = if tool.input_json.trim().is_empty() {
                    json!({})
                } else {
                    serde_json::from_str(&tool.input_json).map_err(|error| {
                        AppError::LlmError(format!("Anthropic tool参数JSON无效: {error}"))
                    })?
                };
                push_chunk(
                    state,
                    None,
                    None,
                    Some(vec![ToolCall {
                        id: tool.id,
                        name: tool.name,
                        arguments,
                    }]),
                    None,
                );
            }
        }
        "message_delta" => {
            state.usage.output = event["usage"]["output_tokens"]
                .as_u64()
                .unwrap_or(state.usage.output as u64) as usize;
            let finish = event["delta"]["stop_reason"]
                .as_str()
                .map(map_stop_reason)
                .map(str::to_string);
            let usage = UsageInfo {
                prompt_tokens: state.usage.input,
                completion_tokens: state.usage.output,
                total_tokens: state.usage.input.saturating_add(state.usage.output),
                prompt_cache_hit_tokens: state.usage.cache_hit,
                prompt_cache_miss_tokens: state.usage.cache_miss,
            };
            push_chunk(
                state,
                None,
                None,
                None,
                finish.map(|finish| (finish, usage)),
            );
        }
        "message_stop" => state.finished = true,
        "error" => {
            return Err(AppError::LlmError(format!(
                "Anthropic返回错误: {}",
                event["error"]
            )));
        }
        _ => {}
    }
    Ok(())
}

fn push_chunk(
    state: &mut AnthropicStreamState,
    content: Option<String>,
    reasoning_content: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
    terminal: Option<(String, UsageInfo)>,
) {
    let (finish_reason, usage) = terminal
        .map(|(reason, usage)| (Some(reason), Some(usage)))
        .unwrap_or((None, None));
    state.queued.push_back(Ok(ChatChunk {
        content,
        reasoning_content,
        tool_calls,
        finish_reason,
        index: state.index,
        usage,
    }));
    state.index = state.index.saturating_add(1);
}

fn required_string(value: &Value, key: &str) -> Result<String, AppError> {
    value[key]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| AppError::LlmError(format!("Anthropic响应缺少字符串字段 {key}")))
}

fn map_stop_reason(reason: &str) -> &str {
    match reason {
        "max_tokens" => "length",
        "tool_use" => "tool_calls",
        "end_turn" | "stop_sequence" => "stop",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_text_tool_usage_and_finish_reason() {
        let mut state = AnthropicStreamState::default();
        for event in [
            json!({"type":"message_start","message":{"usage":{"input_tokens":3}}}),
            json!({"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"call-1","name":"edit"}}),
            json!({"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"id\":1}"}}),
            json!({"type":"content_block_stop","index":0}),
            json!({"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":4}}),
        ] {
            project_anthropic_event(&event, &mut state).unwrap();
        }
        assert_eq!(state.queued.len(), 2);
        let tool = state
            .queued
            .pop_front()
            .unwrap()
            .unwrap()
            .tool_calls
            .unwrap()
            .remove(0);
        assert_eq!(tool.arguments, json!({"id": 1}));
        let terminal = state.queued.pop_front().unwrap().unwrap();
        assert_eq!(terminal.finish_reason.as_deref(), Some("tool_calls"));
        assert_eq!(terminal.usage.unwrap().total_tokens, 7);
    }
}
