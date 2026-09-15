use std::collections::HashMap;

use a4n_error::AppError;
use a4n_llm_provider_contract::{ChatChunk, ChatRequest, MessageRole, ToolCall, UsageInfo};
use serde_json::{Map, Value, json};

use super::{empty_chunk, function_definition, required_string};
use crate::http::response::{optional_usize, required_usize};

pub(in crate::http) fn request_body(
    request: &ChatRequest,
    mut body: Map<String, Value>,
) -> Result<Value, AppError> {
    let mut contents = Vec::<Value>::new();
    let mut system = Vec::new();
    let mut call_names = HashMap::new();
    for message in &request.messages {
        if matches!(message.role, MessageRole::System) {
            system.push(json!({"text": message.content}));
            continue;
        }
        let mut parts = Vec::new();
        if matches!(message.role, MessageRole::Tool) {
            let id = message
                .tool_call_id
                .as_deref()
                .ok_or_else(|| AppError::InvalidArgument("工具结果缺少 tool_call_id".into()))?;
            let (name, native_id) = call_names.get(id).ok_or_else(|| {
                AppError::InvalidArgument(format!("Gemini 工具结果找不到调用: {id}"))
            })?;
            let mut response = json!({"name": name, "response": {"output": message.content}});
            if let Some(id) = native_id {
                response["id"] = json!(id);
            }
            parts.push(json!({"functionResponse": response}));
        } else {
            if !message.content.is_empty() {
                parts.push(json!({"text": message.content}));
            }
            for call in message.tool_calls.as_deref().unwrap_or_default() {
                let meta = call
                    .provider_metadata
                    .as_ref()
                    .and_then(|meta| meta.get("gemini"));
                let native_id = meta
                    .and_then(|meta| meta.get("id"))
                    .filter(|id| !id.is_null())
                    .map(|id| required_string(id, "gemini.id"))
                    .transpose()?;
                let mut function = json!({"name": call.name, "args": call.arguments});
                if let Some(id) = native_id {
                    function["id"] = json!(id);
                }
                let mut part = json!({"functionCall": function});
                if let Some(signature) = meta
                    .and_then(|meta| meta.get("thought_signature"))
                    .filter(|signature| !signature.is_null())
                {
                    part["thoughtSignature"] =
                        json!(required_string(signature, "gemini.thought_signature")?);
                }
                call_names.insert(call.id.as_str(), (call.name.as_str(), native_id));
                parts.push(part);
            }
        }
        if parts.is_empty() {
            return Err(AppError::InvalidArgument("Gemini 消息内容不能为空".into()));
        }
        let role = if matches!(message.role, MessageRole::Assistant) {
            "model"
        } else {
            "user"
        };
        if let Some(last) = contents.last_mut().filter(|item| item["role"] == role) {
            last["parts"].as_array_mut().unwrap().extend(parts);
        } else {
            contents.push(json!({"role": role, "parts": parts}));
        }
    }
    body.insert("contents".into(), json!(contents));
    if !system.is_empty() {
        body.insert("systemInstruction".into(), json!({"parts": system}));
    }
    let mut config = match body.remove("generationConfig") {
        Some(Value::Object(config)) => config,
        None => Map::new(),
        Some(_) => {
            return Err(AppError::InvalidArgument(
                "Gemini generationConfig 必须是对象".into(),
            ));
        }
    };
    for key in [
        "temperature",
        "maxOutputTokens",
        "stopSequences",
        "responseMimeType",
        "responseSchema",
        "responseJsonSchema",
        "candidateCount",
    ] {
        if config.contains_key(key) {
            return Err(AppError::InvalidArgument(format!(
                "Gemini generationConfig 不能覆盖系统字段 {key}"
            )));
        }
    }
    if let Some(value) = request.temperature {
        config.insert("temperature".into(), json!(value));
    }
    if let Some(value) = request.max_tokens {
        config.insert("maxOutputTokens".into(), json!(value));
    }
    if let Some(value) = &request.stop_sequences {
        config.insert("stopSequences".into(), json!(value));
    }
    if let Some(format) = &request.response_format {
        if format.r#type != "json_object" {
            return Err(AppError::InvalidArgument(
                "Gemini 请求只支持 json_object 格式".into(),
            ));
        }
        config.insert("responseMimeType".into(), json!("application/json"));
    }
    if !config.is_empty() {
        body.insert("generationConfig".into(), Value::Object(config));
    }
    if let Some(tools) = &request.tools {
        let functions = tools
            .iter()
            .map(|tool| {
                let function = function_definition(tool)?;
                let mut output = json!({"name": function["name"]});
                if let Some(description) = function.get("description") {
                    output["description"] = description.clone();
                }
                if let Some(parameters) = function.get("parameters") {
                    output["parametersJsonSchema"] = parameters.clone();
                }
                Ok(output)
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        body.insert("tools".into(), json!([{"functionDeclarations": functions}]));
    }
    if let Some(choice) = request.tool_choice.as_deref() {
        let config = match choice {
            "auto" => json!({"mode": "AUTO"}),
            "none" => json!({"mode": "NONE"}),
            "required" => json!({"mode": "ANY"}),
            name => json!({"mode": "ANY", "allowedFunctionNames": [name]}),
        };
        body.insert(
            "toolConfig".into(),
            json!({"functionCallingConfig": config}),
        );
    }
    Ok(Value::Object(body))
}

pub(super) struct GeminiStream {
    prefix: String,
    calls: Vec<ToolCall>,
    finished: bool,
}

impl Default for GeminiStream {
    fn default() -> Self {
        Self {
            prefix: uuid::Uuid::new_v4().to_string(),
            calls: Vec::new(),
            finished: false,
        }
    }
}

impl GeminiStream {
    pub(super) fn parse_event(&mut self, event: &Value) -> Result<Vec<ChatChunk>, AppError> {
        if !event["promptFeedback"]["blockReason"].is_null() {
            return Err(AppError::LlmError(format!(
                "Gemini 拒绝请求: {}",
                event["promptFeedback"]
            )));
        }
        let mut chunks = Vec::new();
        if let Some(candidates) = event.get("candidates") {
            let candidates = candidates
                .as_array()
                .ok_or_else(|| AppError::LlmError("Gemini candidates 必须是数组".into()))?;
            if candidates.len() > 1 {
                return Err(AppError::LlmError("Gemini 返回了多个候选响应".into()));
            }
            if let Some(candidate) = candidates.first() {
                if self.finished {
                    return Err(AppError::LlmError("Gemini 终态后收到额外候选响应".into()));
                }
                if let Some(parts) = candidate.pointer("/content/parts") {
                    let parts = parts
                        .as_array()
                        .ok_or_else(|| AppError::LlmError("Gemini parts 必须是数组".into()))?;
                    for part in parts {
                        if let Some(text) = part.get("text") {
                            let text = text.as_str().ok_or_else(|| {
                                AppError::LlmError("Gemini text 必须是字符串".into())
                            })?;
                            let mut chunk = empty_chunk();
                            if part["thought"] == true {
                                chunk.reasoning_content = Some(text.into());
                            } else {
                                chunk.content = Some(text.into());
                            }
                            chunks.push(chunk);
                        }
                        if let Some(function) = part.get("functionCall") {
                            let args = match function.get("args") {
                                Some(Value::Object(args)) => Value::Object(args.clone()),
                                None => json!({}),
                                Some(_) => {
                                    return Err(AppError::LlmError(
                                        "Gemini functionCall.args 必须是对象".into(),
                                    ));
                                }
                            };
                            let id = function
                                .get("id")
                                .map(|id| required_string(id, "functionCall.id"))
                                .transpose()?;
                            let signature = part
                                .get("thoughtSignature")
                                .map(|value| required_string(value, "thoughtSignature"))
                                .transpose()?;
                            self.calls.push(ToolCall {
                                id: id.map(str::to_string).unwrap_or_else(|| format!("gemini-{}-{}", self.prefix, self.calls.len())),
                                name: required_string(&function["name"], "functionCall.name")?.into(),
                                arguments: args,
                                stream_index: None,
                                provider_metadata: (signature.is_some() || id.is_some()).then(|| json!({"gemini": {"thought_signature": signature, "id": id}})),
                            });
                        }
                    }
                }
                if let Some(finish) = candidate.get("finishReason") {
                    let finish = match finish.as_str() {
                        Some("STOP") if !self.calls.is_empty() => "tool_calls",
                        Some("STOP") => "stop",
                        Some("MAX_TOKENS") => "max_tokens",
                        _ => {
                            return Err(AppError::LlmError(format!(
                                "Gemini 生成未正常完成: {finish}"
                            )));
                        }
                    };
                    self.finished = true;
                    let mut chunk = empty_chunk();
                    chunk.finish_reason = Some(finish.into());
                    if finish == "tool_calls" {
                        chunk.tool_calls = Some(std::mem::take(&mut self.calls));
                    }
                    chunks.push(chunk);
                }
            }
        }
        if let Some(usage) = event.get("usageMetadata") {
            let prompt =
                required_usize(&usage["promptTokenCount"], "usageMetadata.promptTokenCount")?;
            let total = required_usize(&usage["totalTokenCount"], "usageMetadata.totalTokenCount")?;
            let completion = total.checked_sub(prompt).ok_or_else(|| {
                AppError::LlmError("Gemini totalTokenCount 小于 promptTokenCount".into())
            })?;
            let mut chunk = empty_chunk();
            chunk.usage = Some(
                UsageInfo {
                    prompt_tokens: prompt,
                    completion_tokens: completion,
                    total_tokens: total,
                    prompt_cache_hit_tokens: optional_usize(
                        usage.get("cachedContentTokenCount"),
                        "usageMetadata.cachedContentTokenCount",
                    )?,
                    prompt_cache_miss_tokens: None,
                    prompt_cache_write_tokens: None,
                }
                .normalize_cache_tokens()
                .map_err(AppError::LlmError)?,
            );
            chunks.push(chunk);
        }
        Ok(chunks)
    }
}
