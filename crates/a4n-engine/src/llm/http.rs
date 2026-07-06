//! HTTP LLM Provider：通过 OpenAI 兼容 API 调用大语言模型
//!
//! 支持 OpenAI、Ollama、vLLM 等所有兼容 `/chat/completions` 端点的服务。
//! 提供同步聊天、流式聊天（SSE）、连接测试。

use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use reqwest::{Client, StatusCode, header::CONTENT_ENCODING};
use serde_json::{Map, Value, json};
use std::collections::VecDeque;
use std::pin::Pin;
use std::time::{Duration, Instant};

use super::provider::{
    ChatChunk, ChatMessage, ChatRequest, ChatResponse, LlmProvider, MessageRole, ModelCapabilities,
    UsageInfo,
};
use a4n_core::config::app_config::{DEFAULT_LLM_MAX_CONTEXT_TOKENS, DEFAULT_LLM_MAX_OUTPUT_TOKENS};
use a4n_core::error::AppError;

const DEFAULT_TIMEOUT_SECS: u64 = 120;
const MIN_CONNECT_TIMEOUT_SECS: u64 = 10;
const MAX_CONNECT_TIMEOUT_SECS: u64 = 60;
const CONNECTION_TEST_MAX_TOKENS: usize = 8;
const RESERVED_REQUEST_OPTION_KEYS: &[&str] = &[
    "model",
    "messages",
    "stream",
    "temperature",
    "max_tokens",
    "response_format",
    "tools",
    "tool_choice",
    "stop",
];

#[derive(serde::Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

fn internal_error(message: impl Into<String>) -> AppError {
    AppError::Internal(anyhow::anyhow!(message.into()))
}

fn request_error(
    label: &str,
    error: reqwest::Error,
    timeout_secs: u64,
    connect_timeout_secs: u64,
    elapsed: Duration,
) -> AppError {
    if error.is_timeout() || looks_like_timeout(&error) {
        let elapsed = elapsed_seconds_text(elapsed);
        if error.is_connect() {
            return AppError::Timeout(format!(
                "{label}连接超过 {connect_timeout_secs} 秒（实际等待约 {elapsed}；连接超时由 LLM 配置 timeout_secs={timeout_secs} 秒派生，上限 {MAX_CONNECT_TIMEOUT_SECS} 秒），请检查 LLM 服务地址、网络连通性、代理配置或本地模型服务是否可访问"
            ));
        }
        return AppError::Timeout(format!(
            "{label}等待响应超时（实际等待约 {elapsed}，LLM 配置 timeout_secs={timeout_secs} 秒；底层错误：{error}），请检查上游服务 deadline/代理超时，或增大 LLM 配置超时时间并降低本次生成的输出复杂度"
        ));
    }

    internal_error(format!("{label}请求失败: {error}"))
}

fn stream_read_error(error: &reqwest::Error, read_timeout_secs: u64) -> AppError {
    if error.is_timeout() || looks_like_timeout(error) {
        return AppError::Timeout(format!(
            "LLM 流式响应在 {read_timeout_secs} 秒内没有收到新数据，请增大模型配置超时或检查模型服务是否仍在生成"
        ));
    }

    internal_error(format!("流读取错误: {error}"))
}

fn normalized_timeout_secs(timeout_secs: u64) -> u64 {
    timeout_secs.max(1)
}

fn normalized_connect_timeout_secs(timeout_secs: u64) -> u64 {
    timeout_secs.clamp(MIN_CONNECT_TIMEOUT_SECS, MAX_CONNECT_TIMEOUT_SECS)
}

fn elapsed_seconds_text(duration: Duration) -> String {
    let seconds = duration.as_secs_f64();
    if seconds < 1.0 {
        format!("{seconds:.2} 秒")
    } else {
        format!("{seconds:.1} 秒")
    }
}

pub fn normalize_request_options_json(value: Option<Value>) -> Result<Option<Value>, AppError> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let Value::Object(map) = value else {
        return Err(AppError::ConfigError(
            "LLM request_options_json 必须是 JSON 对象".to_string(),
        ));
    };
    for key in map.keys() {
        if RESERVED_REQUEST_OPTION_KEYS
            .iter()
            .any(|reserved| key.eq_ignore_ascii_case(reserved))
        {
            return Err(AppError::ConfigError(format!(
                "LLM request_options_json 不能包含系统保留字段：{key}"
            )));
        }
    }
    Ok((!map.is_empty()).then_some(Value::Object(map)))
}

fn normalize_request_options(
    request_options: Option<Value>,
) -> Result<Option<Map<String, Value>>, AppError> {
    match normalize_request_options_json(request_options)? {
        Some(Value::Object(map)) => Ok(Some(map)),
        _ => Ok(None),
    }
}

fn insert_optional<T: serde::Serialize>(
    body: &mut Map<String, Value>,
    key: &str,
    value: Option<T>,
) {
    if let Some(value) = value {
        body.insert(key.to_string(), json!(value));
    }
}

fn build_http_client(timeout_secs: u64, connect_timeout_secs: u64) -> Client {
    Client::builder()
        .connect_timeout(Duration::from_secs(connect_timeout_secs))
        .read_timeout(Duration::from_secs(timeout_secs))
        .build()
        .expect("创建 HTTP 客户端失败")
}

fn looks_like_timeout(error: &reqwest::Error) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    message.contains("timed out")
        || message.contains("timeout")
        || message.contains("deadline")
        || message.contains("operation timed out")
}

fn response_meta(resp: &reqwest::Response) -> String {
    let encoding = resp
        .headers()
        .get(CONTENT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("none");
    let content_length = resp
        .content_length()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    format!(
        "status={}, content_encoding={}, content_length={}",
        resp.status(),
        encoding,
        content_length
    )
}

fn response_read_error(
    label: &str,
    error: reqwest::Error,
    timeout_secs: u64,
    meta: &str,
) -> AppError {
    if error.is_timeout() || looks_like_timeout(&error) {
        return AppError::Timeout(format!(
            "{label}响应读取超过 {timeout_secs} 秒，请增大 LLM 配置超时时间；{meta}"
        ));
    }
    if error.is_decode() {
        return internal_error(format!(
            "{label}响应体解码失败: {error}; {meta}; 如果失败耗时接近 {timeout_secs} 秒，通常是模型生成过慢、服务端提前断开或压缩响应不完整"
        ));
    }

    internal_error(format!("{label}响应读取失败: {error}; {meta}"))
}

async fn response_text(
    resp: reqwest::Response,
    label: &str,
    timeout_secs: u64,
) -> Result<String, AppError> {
    let meta = response_meta(&resp);
    resp.text()
        .await
        .map_err(|e| response_read_error(label, e, timeout_secs, &meta))
}

fn response_preview(body: &str, max_chars: usize) -> String {
    let mut preview = body.chars().take(max_chars).collect::<String>();
    if body.chars().count() > max_chars {
        preview.push('…');
    }
    preview
}

fn json_response_preview(value: &Value, max_chars: usize) -> String {
    response_preview(&value.to_string(), max_chars)
}

fn parse_response_json(body: &str, label: &str) -> Result<Value, AppError> {
    serde_json::from_str::<Value>(body).map_err(|e| {
        internal_error(format!(
            "解析{label}响应 JSON 失败: {e}; 响应片段: {}",
            response_preview(body, 500)
        ))
    })
}

fn required_object<'a>(
    value: &'a Value,
    field: &str,
) -> Result<&'a serde_json::Map<String, Value>, AppError> {
    value
        .as_object()
        .ok_or_else(|| internal_error(format!("LLM 响应缺少对象字段 {field}")))
}

fn required_string<'a>(value: &'a Value, field: &str) -> Result<&'a str, AppError> {
    value
        .as_str()
        .ok_or_else(|| internal_error(format!("LLM 响应缺少字符串字段 {field}")))
}

fn required_usize(value: &Value, field: &str) -> Result<usize, AppError> {
    let number = value
        .as_u64()
        .ok_or_else(|| internal_error(format!("LLM 响应缺少非负整数字段 {field}")))?;
    usize::try_from(number)
        .map_err(|_| internal_error(format!("LLM 响应字段 {field} 超出 usize 范围")))
}

fn optional_usize(value: Option<&Value>, field: &str) -> Result<Option<usize>, AppError> {
    value.map(|value| required_usize(value, field)).transpose()
}

fn optional_nested_usize(
    value: &Value,
    path: &[&str],
    field: &str,
) -> Result<Option<usize>, AppError> {
    let mut current = value;
    for key in path {
        let Some(next) = current.get(*key) else {
            return Ok(None);
        };
        current = next;
    }
    required_usize(current, field).map(Some)
}

fn non_empty_string(value: &Value) -> Option<&str> {
    value.as_str().map(str::trim).filter(|s| !s.is_empty())
}

fn upstream_error_message(json: &Value) -> Option<String> {
    if let Some(error) = json.get("error") {
        return match error {
            Value::String(message) => {
                let message = message.trim();
                (!message.is_empty()).then(|| message.to_string())
            }
            Value::Object(error) => {
                let message = error.get("message").and_then(non_empty_string);
                let error_type = error.get("type").and_then(non_empty_string);
                let code = error.get("code").and_then(non_empty_string);

                let mut parts = Vec::new();
                if let Some(message) = message {
                    parts.push(message.to_string());
                }
                if let Some(error_type) = error_type {
                    parts.push(format!("type={error_type}"));
                }
                if let Some(code) = code {
                    parts.push(format!("code={code}"));
                }

                if parts.is_empty() {
                    Some(json_response_preview(&Value::Object(error.clone()), 500))
                } else {
                    Some(parts.join("; "))
                }
            }
            other => Some(json_response_preview(other, 500)),
        };
    }

    json.get("message")
        .and_then(non_empty_string)
        .map(|message| {
            if let Some(code) = json.get("code").and_then(non_empty_string) {
                format!("{message}; code={code}")
            } else {
                message.to_string()
            }
        })
}

fn llm_response_shape_error(json: &Value, message: String, body: Option<&str>) -> AppError {
    let mut details = message;
    if let Some(upstream_error) = upstream_error_message(json) {
        details.push_str("；上游错误：");
        details.push_str(&upstream_error);
    }
    details.push_str("；响应片段：");
    match body {
        Some(body) => details.push_str(&response_preview(body, 800)),
        None => details.push_str(&json_response_preview(json, 800)),
    }
    internal_error(details)
}

fn upstream_error_detail(body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|json| upstream_error_message(&json))
        .unwrap_or_else(|| response_preview(body, 800))
}

fn llm_api_status_error(status: StatusCode, body: &str) -> AppError {
    let detail = upstream_error_detail(body);
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => AppError::ConfigError(format!(
            "默认 LLM 配置鉴权失败（{status}）：{detail}。请在设置页更新 API Key、Base URL 或模型名称"
        )),
        StatusCode::PAYMENT_REQUIRED => AppError::ConfigError(format!(
            "默认 LLM 配置对应账号余额不足或额度耗尽（{status}）：{detail}。请充值该供应商账号，或在设置页切换到可用的默认 LLM 配置"
        )),
        StatusCode::TOO_MANY_REQUESTS => AppError::LlmError(format!(
            "LLM 服务限流（{status}）：{detail}。请稍后重试或降低并发"
        )),
        StatusCode::BAD_REQUEST => {
            AppError::LlmError(format!("LLM 请求被上游拒绝（{status}）：{detail}"))
        }
        _ => AppError::LlmError(format!("LLM API 返回错误 {status}：{detail}")),
    }
}

fn stream_usage_only_chunk(json: &Value) -> Result<Option<UsageInfo>, AppError> {
    if !json
        .get("choices")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        return Ok(None);
    }
    stream_usage(json)
}

fn stream_usage(json: &Value) -> Result<Option<UsageInfo>, AppError> {
    let Some(usage) = json.get("usage").filter(|usage| usage.is_object()) else {
        return Ok(None);
    };
    upstream_error_message(json)
        .is_none()
        .then(|| usage_info_from_json(usage))
        .transpose()
}

fn first_choice<'a>(
    json: &'a Value,
    field: &str,
    body: Option<&str>,
) -> Result<&'a Value, AppError> {
    let choices = json["choices"].as_array().ok_or_else(|| {
        llm_response_shape_error(json, format!("LLM 响应缺少数组字段 {field}"), body)
    })?;
    choices.first().ok_or_else(|| {
        llm_response_shape_error(json, format!("LLM 响应字段 {field} 不能为空"), body)
    })
}

fn usage_info_from_json(value: &Value) -> Result<UsageInfo, AppError> {
    let prompt_tokens = required_usize(&value["prompt_tokens"], "usage.prompt_tokens")?;
    let completion_tokens = required_usize(&value["completion_tokens"], "usage.completion_tokens")?;
    let total_tokens = required_usize(&value["total_tokens"], "usage.total_tokens")?;
    let prompt_cache_hit_tokens = prompt_cache_hit_tokens_from_usage(value)?;
    let prompt_cache_miss_tokens = match optional_usize(
        value.get("prompt_cache_miss_tokens"),
        "usage.prompt_cache_miss_tokens",
    )? {
        Some(value) => Some(value),
        None => prompt_cache_hit_tokens.map(|hit| prompt_tokens.saturating_sub(hit)),
    };

    Ok(UsageInfo {
        prompt_tokens,
        completion_tokens,
        total_tokens,
        prompt_cache_hit_tokens,
        prompt_cache_miss_tokens,
    })
}

fn prompt_cache_hit_tokens_from_usage(value: &Value) -> Result<Option<usize>, AppError> {
    if let Some(tokens) = optional_usize(
        value.get("prompt_cache_hit_tokens"),
        "usage.prompt_cache_hit_tokens",
    )? {
        return Ok(Some(tokens));
    }
    if let Some(tokens) = optional_nested_usize(
        value,
        &["prompt_tokens_details", "cached_tokens"],
        "usage.prompt_tokens_details.cached_tokens",
    )? {
        return Ok(Some(tokens));
    }
    optional_nested_usize(
        value,
        &["input_tokens_details", "cached_tokens"],
        "usage.input_tokens_details.cached_tokens",
    )
}

fn parse_sse_data_line(line_bytes: &[u8]) -> Option<Result<ChatChunk, AppError>> {
    let line_bytes = line_bytes.strip_suffix(b"\r").unwrap_or(line_bytes);
    let line = match std::str::from_utf8(line_bytes) {
        Ok(line) => line.trim(),
        Err(e) => {
            return Some(Err(AppError::Internal(anyhow::anyhow!(
                "解析流式响应 UTF-8 失败: {e}"
            ))));
        }
    };

    if line.is_empty() || line.starts_with(':') || !line.starts_with("data:") {
        return None;
    }

    let data = line.strip_prefix("data:").unwrap().trim();
    if data.is_empty() || data == "[DONE]" {
        return None;
    }

    match serde_json::from_str::<Value>(data) {
        Ok(json) => {
            match stream_usage_only_chunk(&json) {
                Ok(Some(usage)) => {
                    return Some(Ok(ChatChunk {
                        content: None,
                        reasoning_content: None,
                        tool_calls: None,
                        finish_reason: None,
                        index: 0,
                        usage: Some(usage),
                    }));
                }
                Ok(None) => {}
                Err(e) => return Some(Err(e)),
            }

            let choice = match first_choice(&json, "choices", None) {
                Ok(choice) => choice,
                Err(e) => return Some(Err(e)),
            };
            let usage = match stream_usage(&json) {
                Ok(usage) => usage,
                Err(e) => return Some(Err(e)),
            };
            if let Err(e) = required_object(&choice["delta"], "choices[0].delta") {
                return Some(Err(e));
            }
            let delta = &choice["delta"];
            let content = delta["content"].as_str().map(|s| s.to_string());
            let reasoning_content = delta["reasoning_content"].as_str().map(|s| s.to_string());
            let finish_reason = choice["finish_reason"].as_str().map(|s| s.to_string());
            let index = match required_usize(&choice["index"], "choices[0].index") {
                Ok(index) => match u32::try_from(index) {
                    Ok(index) => index,
                    Err(_) => {
                        return Some(Err(internal_error(
                            "LLM 响应字段 choices[0].index 超出 u32 范围",
                        )));
                    }
                },
                Err(e) => return Some(Err(e)),
            };

            Some(Ok(ChatChunk {
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
            Some(Err(internal_error(format!(
                "解析流式响应 JSON 失败: {e}; data={preview}"
            ))))
        }
    }
}

fn drain_sse_lines(buffer: &mut Vec<u8>, queue: &mut VecDeque<Result<ChatChunk, AppError>>) {
    while let Some(pos) = buffer.iter().position(|b| *b == b'\n') {
        let mut line = buffer.drain(..=pos).collect::<Vec<_>>();
        if line.last() == Some(&b'\n') {
            line.pop();
        }
        if let Some(item) = parse_sse_data_line(&line) {
            queue.push_back(item);
        }
    }
}

fn enable_stream_usage(body: &mut Map<String, Value>) {
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

/// HTTP LLM Provider
///
/// 通过 HTTP 调用 OpenAI 兼容 API（chat/completions 端点）。
pub struct HttpLlmProvider {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    model: String,
    timeout_secs: u64,
    connect_timeout_secs: u64,
    request_options: Option<Map<String, Value>>,
    caps: ModelCapabilities,
}

impl HttpLlmProvider {
    /// 手动创建（用于测试或自定义配置）
    pub fn new(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
    ) -> Self {
        Self::new_with_timeout(base_url, model, api_key, DEFAULT_TIMEOUT_SECS)
    }

    /// 手动创建并指定非流式请求总超时。
    ///
    /// 流式请求不会使用总超时，因为 reqwest 的总超时会覆盖整个响应体读取，
    /// 长文本生成即使持续输出也可能在固定时间后被强制中断。
    pub fn new_with_timeout(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
        timeout_secs: u64,
    ) -> Self {
        let timeout_secs = normalized_timeout_secs(timeout_secs);
        let connect_timeout_secs = normalized_connect_timeout_secs(timeout_secs);
        let client = build_http_client(timeout_secs, connect_timeout_secs);

        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
            model: model.into(),
            timeout_secs,
            connect_timeout_secs,
            request_options: None,
            caps: ModelCapabilities {
                function_calling: true,
                vision: true,
                max_context_tokens: DEFAULT_LLM_MAX_CONTEXT_TOKENS,
                max_output_tokens: DEFAULT_LLM_MAX_OUTPUT_TOKENS,
            },
        }
    }

    /// 手动创建并追加 provider-specific JSON 请求字段。
    pub fn new_with_timeout_and_options(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
        timeout_secs: u64,
        request_options: Option<Value>,
    ) -> Result<Self, AppError> {
        let mut provider = Self::new_with_timeout(base_url, model, api_key, timeout_secs);
        provider.request_options = normalize_request_options(request_options)?;
        Ok(provider)
    }

    pub fn new_with_timeout_options_and_capabilities(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
        timeout_secs: u64,
        request_options: Option<Value>,
        caps: ModelCapabilities,
    ) -> Result<Self, AppError> {
        let mut provider = Self::new_with_timeout_and_options(
            base_url,
            model,
            api_key,
            timeout_secs,
            request_options,
        )?;
        provider.caps = caps;
        Ok(provider)
    }

    fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }

    /// 构建请求
    fn build_request(&self, request: &ChatRequest, stream: bool) -> reqwest::RequestBuilder {
        let messages: Vec<OpenAiMessage> = request
            .messages
            .iter()
            .map(|m| OpenAiMessage {
                role: match m.role {
                    MessageRole::System => "system".into(),
                    MessageRole::User => "user".into(),
                    MessageRole::Assistant => "assistant".into(),
                    MessageRole::Tool => "tool".into(),
                },
                content: m.content.clone(),
                tool_calls: m.tool_calls.as_ref().map(|tc| {
                    tc.iter()
                        .map(|t| {
                            serde_json::json!({
                                "id": t.id,
                                "type": "function",
                                "function": {
                                    "name": t.name,
                                    "arguments": t.arguments.to_string()
                                }
                            })
                        })
                        .collect()
                }),
                tool_call_id: m.tool_call_id.clone(),
            })
            .collect();

        let mut body = self.request_options.clone().unwrap_or_default();
        body.insert("model".into(), Value::String(self.model.clone()));
        body.insert("messages".into(), json!(messages));
        insert_optional(&mut body, "temperature", request.temperature);
        insert_optional(&mut body, "max_tokens", request.max_tokens);
        insert_optional(
            &mut body,
            "response_format",
            request.response_format.as_ref(),
        );
        insert_optional(&mut body, "tools", request.tools.as_ref());
        insert_optional(&mut body, "tool_choice", request.tool_choice.as_deref());
        insert_optional(&mut body, "stop", request.stop_sequences.as_ref());
        if stream {
            body.insert("stream".into(), Value::Bool(true));
            if request.stream_include_usage {
                enable_stream_usage(&mut body);
            }
        }

        let mut req = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .json(&Value::Object(body));

        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }

        if !stream {
            req = req.timeout(self.request_timeout());
        }

        req
    }
}

impl std::fmt::Debug for HttpLlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpLlmProvider")
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .finish()
    }
}

#[async_trait]
impl LlmProvider for HttpLlmProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError> {
        let request_started = Instant::now();
        let resp = self
            .build_request(&request, false)
            .send()
            .await
            .map_err(|e| {
                request_error(
                    "LLM API",
                    e,
                    self.timeout_secs,
                    self.connect_timeout_secs,
                    request_started.elapsed(),
                )
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = response_text(resp, "LLM API", self.timeout_secs).await?;
            return Err(llm_api_status_error(status, &body));
        }

        let body = response_text(resp, "LLM API", self.timeout_secs).await?;
        let json = parse_response_json(&body, "LLM API")?;

        let choice = first_choice(&json, "choices", Some(&body))?;
        let msg = &choice["message"];
        required_object(msg, "choices[0].message")?;

        let content = required_string(&msg["content"], "choices[0].message.content")?.to_string();
        let finish_reason = choice["finish_reason"].as_str().map(|s| s.to_string());

        let usage = json.get("usage").map(usage_info_from_json).transpose()?;

        Ok(ChatResponse {
            message: ChatMessage {
                role: MessageRole::Assistant,
                content,
                tool_calls: None,
                tool_call_id: None,
            },
            usage,
            finish_reason,
        })
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        let request_started = Instant::now();
        let resp = self
            .build_request(&request, true)
            .send()
            .await
            .map_err(|e| {
                request_error(
                    "LLM API 流式",
                    e,
                    self.timeout_secs,
                    self.connect_timeout_secs,
                    request_started.elapsed(),
                )
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = response_text(resp, "LLM API", self.timeout_secs).await?;
            return Err(llm_api_status_error(status, &body));
        }

        let byte_stream = Box::pin(resp.bytes_stream());
        let read_timeout_secs = self.timeout_secs;
        let stream = futures::stream::unfold(
            (
                byte_stream,
                Vec::<u8>::new(),
                VecDeque::<Result<ChatChunk, AppError>>::new(),
                false,
            ),
            move |(mut byte_stream, mut buffer, mut queue, mut finished)| async move {
                loop {
                    if let Some(item) = queue.pop_front() {
                        return Some((item, (byte_stream, buffer, queue, finished)));
                    }

                    if finished {
                        return None;
                    }

                    match byte_stream.next().await {
                        Some(Ok(bytes)) => {
                            buffer.extend_from_slice(&bytes);
                            drain_sse_lines(&mut buffer, &mut queue);
                        }
                        Some(Err(e)) => {
                            finished = true;
                            return Some((
                                Err(stream_read_error(&e, read_timeout_secs)),
                                (byte_stream, buffer, queue, finished),
                            ));
                        }
                        None => {
                            finished = true;
                            if !buffer.is_empty() {
                                let tail = std::mem::take(&mut buffer);
                                if let Some(item) = parse_sse_data_line(&tail) {
                                    queue.push_back(item);
                                    continue;
                                }
                            }
                        }
                    }
                }
            },
        );

        Ok(Box::pin(stream))
    }

    async fn test_connection(&self) -> Result<bool, AppError> {
        // 发送一个最小请求来测试连接
        let test_request = ChatRequest {
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: "ping".into(),
                tool_calls: None,
                tool_call_id: None,
            }],
            temperature: Some(0.0),
            max_tokens: Some(CONNECTION_TEST_MAX_TOKENS),
            max_tokens_profile: None,
            response_format: None,
            stream_include_usage: false,
            trace_context: None,
            context_manifest_json: None,
            tools: None,
            tool_choice: None,
            stop_sequences: None,
        };

        self.chat(test_request).await.map(|_| true)
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn capabilities(&self) -> ModelCapabilities {
        self.caps.clone()
    }

    fn count_tokens(&self, text: &str) -> usize {
        // 粗略估算：中文每字约 0.5 token，英文每词约 1.3 token
        let chinese_chars = text.chars().filter(|c| u32::from(*c) > 0x2E80).count();
        let other_words = if chinese_chars == text.len() {
            0
        } else {
            text.split_whitespace().count()
        };
        ((chinese_chars as f32 * 0.5) + (other_words as f32 * 1.3)).ceil() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse_data_line(line: &str) -> Result<ChatChunk, AppError> {
        parse_sse_data_line(line.as_bytes()).expect("data line should produce a parse result")
    }

    fn request_body_json(request: reqwest::RequestBuilder) -> Value {
        let request = request.build().expect("request should build");
        let body = request
            .body()
            .and_then(reqwest::Body::as_bytes)
            .expect("request should have an in-memory JSON body");
        serde_json::from_slice(body).expect("request body should be JSON")
    }

    #[test]
    fn parse_sse_data_line_accepts_valid_chunk() {
        let chunk = parse_data_line(
            r#"data: {"choices":[{"index":2,"delta":{"content":"你好"},"finish_reason":"stop"}]}"#,
        )
        .expect("valid chunk");

        assert_eq!(chunk.content.as_deref(), Some("你好"));
        assert_eq!(chunk.reasoning_content.as_deref(), None);
        assert_eq!(chunk.finish_reason.as_deref(), Some("stop"));
        assert_eq!(chunk.index, 2);
        assert!(chunk.usage.is_none());
    }

    #[test]
    fn parse_sse_data_line_accepts_reasoning_chunk_without_content() {
        let chunk = parse_data_line(
            r#"data: {"choices":[{"index":0,"delta":{"content":null,"reasoning_content":"正在分析"},"finish_reason":null}]}"#,
        )
        .expect("reasoning chunk");

        assert_eq!(chunk.content.as_deref(), None);
        assert_eq!(chunk.reasoning_content.as_deref(), Some("正在分析"));
        assert_eq!(chunk.finish_reason.as_deref(), None);
        assert_eq!(chunk.index, 0);
        assert!(chunk.usage.is_none());
    }

    #[test]
    fn parse_sse_data_line_rejects_empty_choices() {
        let err = parse_data_line(r#"data: {"choices":[]}"#)
            .expect_err("empty choices should be rejected");

        let message = err.to_string();
        assert!(message.contains("choices"));
        assert!(message.contains("响应片段"));
    }

    #[test]
    fn parse_sse_data_line_emits_usage_only_empty_choices_chunk() {
        let chunk = parse_data_line(
            r#"data: {"id":"chatcmpl-test","choices":[],"usage":{"prompt_tokens":12,"completion_tokens":34,"total_tokens":46,"prompt_cache_hit_tokens":10,"prompt_cache_miss_tokens":2}}"#,
        )
        .expect("usage-only chunk should be emitted");

        assert!(chunk.content.is_none());
        assert!(chunk.reasoning_content.is_none());
        let usage = chunk.usage.expect("usage-only chunk should carry usage");
        assert_eq!(usage.prompt_tokens, 12);
        assert_eq!(usage.completion_tokens, 34);
        assert_eq!(usage.total_tokens, 46);
        assert_eq!(usage.prompt_cache_hit_tokens, Some(10));
        assert_eq!(usage.prompt_cache_miss_tokens, Some(2));
    }

    #[test]
    fn parse_sse_data_line_keeps_usage_on_choice_chunk() {
        let chunk = parse_data_line(
            r#"data: {"id":"chatcmpl-test","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":12,"completion_tokens":34,"total_tokens":46,"prompt_cache_hit_tokens":10,"prompt_cache_miss_tokens":2}}"#,
        )
        .expect("choice chunk with usage should parse");

        assert_eq!(chunk.finish_reason.as_deref(), Some("stop"));
        let usage = chunk.usage.expect("choice chunk should carry usage");
        assert_eq!(usage.prompt_tokens, 12);
        assert_eq!(usage.completion_tokens, 34);
        assert_eq!(usage.total_tokens, 46);
        assert_eq!(usage.prompt_cache_hit_tokens, Some(10));
        assert_eq!(usage.prompt_cache_miss_tokens, Some(2));
    }

    #[test]
    fn parse_sse_data_line_rejects_empty_choices_with_upstream_error() {
        let err = parse_data_line(
            r#"data: {"choices":[],"error":{"message":"context length exceeded","code":"context_length_exceeded"}}"#,
        )
        .expect_err("empty choices with error should be rejected");

        let message = err.to_string();
        assert!(message.contains("context length exceeded"));
        assert!(message.contains("code=context_length_exceeded"));
    }

    #[test]
    fn first_choice_reports_upstream_error_for_empty_choices() {
        let body = r#"{"id":"chatcmpl-test","choices":[],"error":{"message":"context length exceeded","type":"invalid_request_error","code":"context_length_exceeded"}}"#;
        let json = parse_response_json(body, "LLM API").expect("valid response json");
        let err =
            first_choice(&json, "choices", Some(body)).expect_err("empty choices should fail");

        let message = err.to_string();
        assert!(message.contains("choices"));
        assert!(message.contains("context length exceeded"));
        assert!(message.contains("type=invalid_request_error"));
        assert!(message.contains("code=context_length_exceeded"));
        assert!(message.contains("chatcmpl-test"));
    }

    #[test]
    fn first_choice_reports_top_level_message_for_empty_choices() {
        let body = r#"{"choices":[],"message":"content filtered by upstream service","code":"content_filter"}"#;
        let json = parse_response_json(body, "LLM API").expect("valid response json");
        let err =
            first_choice(&json, "choices", Some(body)).expect_err("empty choices should fail");

        let message = err.to_string();
        assert!(message.contains("content filtered by upstream service"));
        assert!(message.contains("code=content_filter"));
        assert!(message.contains("响应片段"));
    }

    #[test]
    fn payment_required_status_is_configuration_error() {
        let body = r#"{"error":{"message":"Insufficient Balance","type":"unknown_error","code":"invalid_request_error"}}"#;
        let err = llm_api_status_error(StatusCode::PAYMENT_REQUIRED, body);

        let message = err.to_string();
        assert!(matches!(err, AppError::ConfigError(_)));
        assert!(message.contains("余额不足"));
        assert!(message.contains("Insufficient Balance"));
        assert!(!message.contains("内部错误"));
    }

    #[test]
    fn request_options_reject_reserved_keys() {
        let err = normalize_request_options(Some(json!({
            "model": "override"
        })))
        .expect_err("reserved request option key should be rejected");

        assert!(err.to_string().contains("系统保留字段"));
    }

    #[test]
    fn request_options_reject_response_format_as_request_contract() {
        let err = normalize_request_options(Some(json!({
            "response_format": { "type": "json_object" }
        })))
        .expect_err("response_format is controlled per request");

        assert!(err.to_string().contains("系统保留字段"));
    }

    #[test]
    fn request_options_accept_provider_extension_fields() {
        let options = normalize_request_options(Some(json!({
            "reasoning_effort": "low"
        })))
        .expect("provider extension field should be accepted")
        .expect("non-empty options");

        assert_eq!(options["reasoning_effort"], "low");
    }

    #[test]
    fn connect_timeout_is_derived_from_llm_timeout_with_bounds() {
        assert_eq!(normalized_connect_timeout_secs(1), MIN_CONNECT_TIMEOUT_SECS);
        assert_eq!(normalized_connect_timeout_secs(30), 30);
        assert_eq!(
            normalized_connect_timeout_secs(300),
            MAX_CONNECT_TIMEOUT_SECS
        );
    }

    #[test]
    fn provider_uses_derived_connect_timeout() {
        let provider =
            HttpLlmProvider::new_with_timeout("https://example.test", "deepseek-chat", None, 300);

        assert_eq!(provider.timeout_secs, 300);
        assert_eq!(provider.connect_timeout_secs, MAX_CONNECT_TIMEOUT_SECS);
    }

    #[test]
    fn stream_request_includes_usage_only_when_requested() {
        let provider = HttpLlmProvider::new("https://example.test", "deepseek-chat", None);
        let base_request = ChatRequest::system_user("system", "user", Some(0.2), None, None);

        let without_usage = request_body_json(provider.build_request(&base_request, true));
        assert_eq!(without_usage["stream"], json!(true));
        assert!(without_usage.get("stream_options").is_none());

        let with_usage =
            request_body_json(provider.build_request(&base_request.with_stream_usage(), true));
        assert_eq!(with_usage["stream"], json!(true));
        assert_eq!(with_usage["stream_options"]["include_usage"], json!(true));
    }

    #[test]
    fn stream_request_usage_merges_existing_stream_options() {
        let provider = HttpLlmProvider::new_with_timeout_and_options(
            "https://example.test",
            "deepseek-chat",
            None,
            DEFAULT_TIMEOUT_SECS,
            Some(json!({
                "stream_options": {
                    "existing_provider_option": "kept"
                }
            })),
        )
        .expect("provider options should be accepted");
        let request =
            ChatRequest::system_user("system", "user", Some(0.2), None, None).with_stream_usage();

        let body = request_body_json(provider.build_request(&request, true));
        assert_eq!(body["stream_options"]["include_usage"], json!(true));
        assert_eq!(
            body["stream_options"]["existing_provider_option"],
            json!("kept")
        );
    }

    #[test]
    fn request_includes_json_response_format_when_requested() {
        let provider = HttpLlmProvider::new("https://example.test", "deepseek-chat", None);
        let request = ChatRequest::system_user("system", "user", Some(0.2), None, None)
            .with_json_response_format();

        let body = request_body_json(provider.build_request(&request, false));
        assert_eq!(body["response_format"]["type"], json!("json_object"));
    }

    #[test]
    fn parse_sse_data_line_requires_choice_index() {
        let err = parse_data_line(r#"data: {"choices":[{"delta":{"content":"x"}}]}"#)
            .expect_err("missing index should be rejected");

        assert!(err.to_string().contains("choices[0].index"));
    }

    #[test]
    fn parse_response_json_includes_body_preview_on_invalid_json() {
        let err = parse_response_json("<html>bad gateway</html>", "LLM API")
            .expect_err("invalid json should include response preview");

        let message = err.to_string();
        assert!(message.contains("解析LLM API响应 JSON 失败"));
        assert!(message.contains("<html>bad gateway</html>"));
    }

    #[test]
    fn usage_info_accepts_complete_usage() {
        let usage = usage_info_from_json(&json!({
            "prompt_tokens": 3,
            "completion_tokens": 5,
            "total_tokens": 8
        }))
        .expect("complete usage");

        assert_eq!(usage.prompt_tokens, 3);
        assert_eq!(usage.completion_tokens, 5);
        assert_eq!(usage.total_tokens, 8);
        assert_eq!(usage.prompt_cache_hit_tokens, None);
        assert_eq!(usage.prompt_cache_miss_tokens, None);
    }

    #[test]
    fn usage_info_accepts_deepseek_cache_fields() {
        let usage = usage_info_from_json(&json!({
            "prompt_tokens": 100,
            "completion_tokens": 20,
            "total_tokens": 120,
            "prompt_cache_hit_tokens": 80,
            "prompt_cache_miss_tokens": 20
        }))
        .expect("deepseek usage");

        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.prompt_cache_hit_tokens, Some(80));
        assert_eq!(usage.prompt_cache_miss_tokens, Some(20));
    }

    #[test]
    fn usage_info_accepts_openai_cached_tokens_detail() {
        let usage = usage_info_from_json(&json!({
            "prompt_tokens": 100,
            "completion_tokens": 20,
            "total_tokens": 120,
            "prompt_tokens_details": {
                "cached_tokens": 75
            }
        }))
        .expect("openai usage");

        assert_eq!(usage.prompt_cache_hit_tokens, Some(75));
        assert_eq!(usage.prompt_cache_miss_tokens, Some(25));
    }

    #[test]
    fn usage_info_requires_all_token_fields() {
        let err = usage_info_from_json(&json!({
            "prompt_tokens": 3,
            "total_tokens": 8
        }))
        .expect_err("missing completion token count should be rejected");

        assert!(err.to_string().contains("usage.completion_tokens"));
    }
}
