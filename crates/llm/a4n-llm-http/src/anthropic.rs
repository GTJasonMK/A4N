use std::{collections::HashMap, pin::Pin, time::Instant};

use a4n_error::AppError;
use a4n_llm_provider_contract::{
    ChatChunk, ChatRequest, LlmProvider, MessageRole, ModelCapabilities, ToolCall, UsageInfo,
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::{Map, Value, json};

use super::errors::{
    build_http_client, normalized_connect_timeout_secs, normalized_timeout_secs, request_error,
    response_text,
};
use super::response::{llm_api_status_error, optional_usize, required_usize};

pub(super) struct AnthropicLlmProvider {
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
    pub(super) fn new(
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
            extra_body: a4n_llm_provider_contract::normalize_extra_body(extra_body)?,
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

        let mut system = Vec::new();
        for (index, message) in request.messages.iter().enumerate() {
            if !matches!(message.role, MessageRole::System) || message.content.is_empty() {
                continue;
            }
            let mut blocks = anthropic_text_blocks(request, index)?;
            if !system.is_empty()
                && let Some(first) = blocks.first_mut()
            {
                first["text"] = json!(format!("\n\n{}", first["text"].as_str().unwrap()));
            }
            system.extend(blocks);
        }
        if request.response_format.is_some() {
            let separator = if system.is_empty() { "" } else { "\n\n" };
            system.push(json!({"type": "text", "text": format!("{separator}Return only one valid JSON value. Do not wrap it in Markdown fences or add explanatory text.")}));
        }
        if !system.is_empty() {
            body.insert("system".into(), Value::Array(system));
        }

        body.insert("messages".into(), anthropic_messages(request)?);
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
}

fn anthropic_text_blocks(request: &ChatRequest, index: usize) -> Result<Vec<Value>, AppError> {
    Ok(request
        .cached_text_parts(index)?
        .into_iter()
        .map(|(text, cached)| {
            let mut block = json!({"type": "text", "text": text});
            if cached {
                block["cache_control"] = json!({"type": "ephemeral"});
            }
            block
        })
        .collect())
}

fn anthropic_messages(request: &ChatRequest) -> Result<Value, AppError> {
    let mut output = Vec::new();
    for (index, message) in request.messages.iter().enumerate() {
        if matches!(message.role, MessageRole::System) {
            continue;
        }
        match message.role {
            MessageRole::User => output.push(json!({
                "role": "user",
                "content": anthropic_content_blocks(request, index)?
            })),
            MessageRole::Assistant => {
                let mut blocks = Vec::new();
                if !message.content.is_empty() {
                    blocks.extend(anthropic_text_blocks(request, index)?);
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
                let parts = request.cached_text_parts(index)?;
                if parts.len() > 1 {
                    return Err(AppError::InvalidArgument(
                        "Anthropic 工具结果只支持完整块缓存断点".into(),
                    ));
                }
                let mut block =
                    json!({"type": "tool_result", "tool_use_id": id, "content": message.content});
                if parts[0].1 {
                    block["cache_control"] = json!({"type": "ephemeral"});
                }
                output.push(json!({"role": "user", "content": [block]}));
            }
            MessageRole::System => unreachable!(),
        }
    }
    Ok(Value::Array(output))
}

fn anthropic_content_blocks(request: &ChatRequest, index: usize) -> Result<Vec<Value>, AppError> {
    let mut blocks = anthropic_text_blocks(request, index)?;
    let is_last_user = request
        .messages
        .iter()
        .rposition(|message| matches!(message.role, MessageRole::User))
        == Some(index);
    if !is_last_user {
        return Ok(blocks);
    }
    for image in &request.vision_images {
        if !image.media_type.starts_with("image/") || image.data_base64.is_empty() {
            return Err(AppError::InvalidArgument("视觉输入图片数据无效".into()));
        }
        blocks.push(json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": image.media_type,
                "data": image.data_base64
            }
        }));
    }
    Ok(blocks)
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
    cache_write: Option<usize>,
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
            state.usage.input = required_usize(&usage["input_tokens"], "usage.input_tokens")?;
            state.usage.cache_hit = optional_usize(
                usage.get("cache_read_input_tokens"),
                "usage.cache_read_input_tokens",
            )?;
            state.usage.cache_write = optional_usize(
                usage.get("cache_creation_input_tokens"),
                "usage.cache_creation_input_tokens",
            )?;
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
                        stream_index: Some(index),
                        provider_metadata: None,
                    }]),
                    None,
                );
            }
        }
        "message_delta" => {
            state.usage.output =
                required_usize(&event["usage"]["output_tokens"], "usage.output_tokens")?;
            let finish = event["delta"]["stop_reason"]
                .as_str()
                .map(map_stop_reason)
                .map(str::to_string);
            // Anthropic input_tokens excludes both cached reads and writes.
            let prompt_tokens = state
                .usage
                .input
                .checked_add(state.usage.cache_hit.unwrap_or(0))
                .and_then(|tokens| tokens.checked_add(state.usage.cache_write.unwrap_or(0)))
                .ok_or_else(|| AppError::LlmError("Anthropic 输入 token 数溢出".into()))?;
            let usage = UsageInfo {
                prompt_tokens,
                completion_tokens: state.usage.output,
                total_tokens: prompt_tokens
                    .checked_add(state.usage.output)
                    .ok_or_else(|| AppError::LlmError("Anthropic 总 token 数溢出".into()))?,
                prompt_cache_hit_tokens: state.usage.cache_hit,
                prompt_cache_miss_tokens: None,
                prompt_cache_write_tokens: state.usage.cache_write,
            }
            .normalize_cache_tokens()
            .map_err(AppError::LlmError)?;
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
    use a4n_llm_provider_contract::ChatImage;

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

    #[test]
    fn request_uses_configured_large_model_output_cap() {
        let provider = AnthropicLlmProvider::new(
            "https://example.test",
            "claude-test",
            None,
            120,
            None,
            ModelCapabilities {
                max_context_tokens: 1_048_576,
                max_output_tokens: 393_216,
            },
        )
        .expect("provider should accept model capabilities");
        let request = ChatRequest::system_user("system", "user", None, None);

        let body = provider.request_body(&request).expect("request body");

        assert_eq!(body["max_tokens"], json!(393_216));
        let request = ChatRequest::system_user("", "user", None, None);
        assert!(
            provider
                .request_body(&request)
                .unwrap()
                .get("system")
                .is_none()
        );
    }

    #[test]
    fn request_encodes_vision_images_as_anthropic_base64_blocks() {
        let provider = AnthropicLlmProvider::new(
            "https://example.test",
            "claude-test",
            None,
            30,
            None,
            ModelCapabilities {
                max_context_tokens: 4096,
                max_output_tokens: 1024,
            },
        )
        .unwrap();
        let mut request = ChatRequest::system_user("system", "检查这张图", None, Some(100));
        request.vision_images = vec![ChatImage {
            media_type: "image/png".into(),
            data_base64: "aGVsbG8".into(),
        }];
        let body = provider.request_body(&request).unwrap();
        assert_eq!(body["messages"][0]["content"][1]["type"], "image");
        assert_eq!(
            body["messages"][0]["content"][1]["source"]["data"],
            "aGVsbG8"
        );
    }

    #[test]
    fn claude_cache_uses_system_and_user_boundaries_and_normalizes_all_input() {
        let provider = AnthropicLlmProvider::new(
            "https://example.test",
            "claude-test",
            None,
            120,
            None,
            ModelCapabilities {
                max_context_tokens: 100_000,
                max_output_tokens: 8192,
            },
        )
        .unwrap();
        let mut request = ChatRequest::system_user("rules", "shared\nnew task", None, None);
        request.add_cache_breakpoint(1, "shared".len()).unwrap();
        request
            .configure_prompt_cache("anthropic", "claude-test", None, Some(7))
            .unwrap();
        let body = provider.request_body(&request).unwrap();
        assert_eq!(body["system"][0]["cache_control"]["type"], "ephemeral");
        assert_eq!(
            body["messages"][0]["content"][0]["cache_control"]["type"],
            "ephemeral"
        );
        assert_eq!(body["messages"][0]["content"][1]["text"], "\nnew task");
        let mut state = AnthropicStreamState::default();
        for event in [
            json!({"type":"message_start","message":{"usage":{"input_tokens":1000,"cache_read_input_tokens":9000,"cache_creation_input_tokens":500}}}),
            json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":100}}),
        ] {
            project_anthropic_event(&event, &mut state).unwrap();
        }
        let usage = state.queued.pop_front().unwrap().unwrap().usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10500);
        assert_eq!(usage.total_tokens, 10600);
        assert_eq!(usage.prompt_cache_miss_tokens, Some(1500));
        assert_eq!(usage.prompt_cache_write_tokens, Some(500));
    }
}
