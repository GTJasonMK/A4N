use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_llm::trace::context_aware_chat_request_trace_json;
use serde_json::{Value, json};

pub(crate) fn required_message_content(content: &str) -> Result<&str, AppError> {
    let content = content.trim();
    if content.is_empty() {
        Err(AppError::InvalidArgument("消息内容不能为空".to_string()))
    } else {
        Ok(content)
    }
}

pub(super) fn inspiration_call_fingerprint_json(
    project_id: i64,
    call_key: &str,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": "inspiration_conversation_call_v1",
        "workflow": "inspiration_conversation",
        "call_key": call_key,
        "project_id": project_id,
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}

pub(super) fn validate_inspiration_response_json(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim();
    let value: Value = serde_json::from_str(trimmed).map_err(|err| {
        AppError::InvalidArgument(format!(
            "灵感对话必须返回严格 JSON 对象: {err}; 输出预览：{}",
            inspiration_response_preview(trimmed)
        ))
    })?;
    if !value.is_object() {
        return Err(AppError::InvalidArgument(
            "灵感对话必须返回严格 JSON 对象，不能返回数组、字符串或其他 JSON 类型".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

fn inspiration_response_preview(raw: &str) -> String {
    if raw.is_empty() {
        return "<empty>".to_string();
    }
    raw.chars().take(180).collect()
}
