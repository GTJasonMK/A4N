use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_llm_trace::context_aware_chat_request_trace_json;
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
    a4n_domain::inspiration::parse_inspiration_response(trimmed).map_err(|err| {
        AppError::InvalidArgument(format!(
            "灵感对话必须返回严格 JSON 对象: {err}; 输出预览：{}",
            inspiration_response_preview(trimmed)
        ))
    })?;
    Ok(trimmed.to_string())
}

fn inspiration_response_preview(raw: &str) -> String {
    if raw.is_empty() {
        return "<empty>".to_string();
    }
    raw.chars().take(180).collect()
}
