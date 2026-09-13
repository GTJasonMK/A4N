use a4n_error::AppError;
use a4n_llm::provider::{ChatChunk, ChatRequest, MessageRole, ToolCall, UsageInfo};
use serde_json::{Map, Value, json};

use super::{empty_chunk, function_definition, required_string};
use crate::http::cache::{apply_openai_cache_options, openai_text_content};
use crate::http::response::{optional_nested_usize, required_usize};

pub(in crate::http) fn request_body(
    request: &ChatRequest,
    model: &str,
    mut body: Map<String, Value>,
) -> Result<Value, AppError> {
    if request
        .stop_sequences
        .as_ref()
        .is_some_and(|stop| !stop.is_empty())
    {
        return Err(AppError::InvalidArgument(
            "Responses API 不支持 stop_sequences".into(),
        ));
    }
    let mut input = Vec::new();
    for (index, message) in request.messages.iter().enumerate() {
        if matches!(message.role, MessageRole::Tool) {
            let call_id = message
                .tool_call_id
                .as_deref()
                .filter(|id| !id.is_empty())
                .ok_or_else(|| AppError::InvalidArgument("工具结果缺少 tool_call_id".into()))?;
            input.push(json!({"type": "function_call_output", "call_id": call_id, "output": openai_text_content(request, index, "input_text")?}));
            continue;
        }
        let calls = message.tool_calls.as_deref().unwrap_or_default();
        for call in calls {
            if let Some(reasoning) = call
                .provider_metadata
                .as_ref()
                .and_then(|meta| meta.get("responses_reasoning"))
            {
                let items = reasoning.as_array().ok_or_else(|| {
                    AppError::InvalidArgument("Responses reasoning 必须是数组".into())
                })?;
                if items.iter().any(|item| item["type"] != "reasoning") {
                    return Err(AppError::InvalidArgument(
                        "Responses reasoning 包含非 reasoning 项".into(),
                    ));
                }
                input.extend(items.iter().cloned());
            }
        }
        if !message.content.is_empty() || calls.is_empty() {
            let kind = if matches!(message.role, MessageRole::Assistant) {
                "output_text"
            } else {
                "input_text"
            };
            input.push(json!({"role": message.role, "content": openai_text_content(request, index, kind)?}));
        }
        for call in calls {
            input.push(
                json!({"type": "function_call", "call_id": call.id, "name": call.name,
                "arguments": serde_json::to_string(&call.arguments)?}),
            );
        }
    }
    body.insert("model".into(), json!(model));
    body.insert("input".into(), json!(input));
    body.insert("stream".into(), json!(true));
    // A4N owns conversation persistence. Keep reasoning replayable without
    // introducing a second, provider-hosted conversation state.
    body.insert("store".into(), json!(false));
    body.insert("include".into(), json!(["reasoning.encrypted_content"]));
    if let Some(value) = request.max_tokens {
        body.insert("max_output_tokens".into(), json!(value));
    }
    if let Some(value) = request.temperature {
        body.insert("temperature".into(), json!(value));
    }
    if let Some(format) = &request.response_format {
        if format.r#type != "json_object" {
            return Err(AppError::InvalidArgument(
                "Responses 请求只支持 json_object 格式".into(),
            ));
        }
        body.insert("text".into(), json!({"format": {"type": "json_object"}}));
    }
    apply_openai_cache_options(request, &mut body)?;
    if let Some(tools) = &request.tools {
        let tools = tools
            .iter()
            .map(|tool| {
                let mut function = function_definition(tool)?.as_object().unwrap().clone();
                function.insert("type".into(), json!("function"));
                // Preserve the existing, non-strict Chat tool schemas. Responses
                // defaults to strict mode, which changes optional-field semantics.
                function.entry("strict").or_insert(json!(false));
                Ok(Value::Object(function))
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        body.insert("tools".into(), json!(tools));
    }
    if let Some(choice) = &request.tool_choice {
        let choice = match choice.as_str() {
            "auto" | "none" | "required" => json!(choice),
            name => json!({"type": "function", "name": name}),
        };
        body.insert("tool_choice".into(), choice);
    }
    Ok(Value::Object(body))
}

pub(super) fn parse_event(event: &Value) -> Result<Vec<ChatChunk>, AppError> {
    let kind = required_string(&event["type"], "type")?;
    let delta = || {
        event["delta"]
            .as_str()
            .ok_or_else(|| AppError::LlmError("Responses delta 必须是字符串".into()))
    };
    let mut chunk = empty_chunk();
    match kind {
        "response.output_text.delta" => {
            chunk.content = Some(delta()?.into());
        }
        "response.reasoning_summary_text.delta" | "response.reasoning_text.delta" => {
            chunk.reasoning_content = Some(delta()?.into());
        }
        "response.completed" | "response.incomplete" => {
            return terminal_chunks(&event["response"], kind);
        }
        "response.failed" | "response.cancelled" | "error" => {
            return Err(AppError::LlmError(format!("Responses 请求失败: {event}")));
        }
        "response.refusal.delta" => {
            return Err(AppError::LlmError(format!(
                "Responses 拒绝请求: {}",
                event["delta"]
            )));
        }
        // Function arguments are emitted once from the authoritative completed
        // output, together with encrypted reasoning needed for the next turn.
        _ => return Ok(Vec::new()),
    }
    Ok(vec![chunk])
}

fn terminal_chunks(response: &Value, event_kind: &str) -> Result<Vec<ChatChunk>, AppError> {
    let status = required_string(&response["status"], "response.status")?;
    let finish = match (event_kind, status) {
        ("response.completed", "completed") => "stop",
        ("response.incomplete", "incomplete") => {
            match response["incomplete_details"]["reason"].as_str() {
                Some("max_output_tokens") => "max_tokens",
                Some("content_filter") => "content_filter",
                _ => {
                    return Err(AppError::LlmError(format!(
                        "Responses 未完成: {}",
                        response["incomplete_details"]
                    )));
                }
            }
        }
        _ => return Err(AppError::LlmError("Responses 事件与终态不一致".into())),
    };
    let output = response["output"]
        .as_array()
        .ok_or_else(|| AppError::LlmError("Responses 缺少 output 数组".into()))?;
    let reasoning: Vec<_> = output
        .iter()
        .filter(|item| item["type"] == "reasoning")
        .cloned()
        .collect();
    let mut calls = Vec::new();
    if finish == "stop" {
        for item in output.iter().filter(|item| item["type"] == "function_call") {
            calls.push(ToolCall {
                id: required_string(&item["call_id"], "function_call.call_id")?.into(),
                name: required_string(&item["name"], "function_call.name")?.into(),
                arguments: serde_json::from_str(required_string(
                    &item["arguments"],
                    "function_call.arguments",
                )?)
                .map_err(|error| {
                    AppError::LlmError(format!("Responses 工具参数 JSON 无效: {error}"))
                })?,
                stream_index: None,
                provider_metadata: (calls.is_empty() && !reasoning.is_empty())
                    .then(|| json!({"responses_reasoning": reasoning})),
            });
        }
    }
    let mut chunk = empty_chunk();
    chunk.finish_reason = Some(
        if !calls.is_empty() {
            "tool_calls"
        } else {
            finish
        }
        .into(),
    );
    chunk.tool_calls = (!calls.is_empty()).then_some(calls);
    if !response["usage"].is_null() {
        let usage = &response["usage"];
        chunk.usage = Some(
            UsageInfo {
                prompt_tokens: required_usize(&usage["input_tokens"], "usage.input_tokens")?,
                completion_tokens: required_usize(&usage["output_tokens"], "usage.output_tokens")?,
                total_tokens: required_usize(&usage["total_tokens"], "usage.total_tokens")?,
                prompt_cache_hit_tokens: optional_nested_usize(
                    usage,
                    &["input_tokens_details", "cached_tokens"],
                    "usage.input_tokens_details.cached_tokens",
                )?,
                prompt_cache_miss_tokens: None,
                prompt_cache_write_tokens: optional_nested_usize(
                    usage,
                    &["input_tokens_details", "cache_write_tokens"],
                    "usage.input_tokens_details.cache_write_tokens",
                )?,
            }
            .normalize_cache_tokens()
            .map_err(AppError::LlmError)?,
        );
    }
    Ok(vec![chunk])
}
