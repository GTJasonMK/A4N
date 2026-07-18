use a4n_llm::provider::{ChatMessage, ChatRequest, ResponseFormat};
use serde_json::Value;

pub(crate) fn chat_request_from_preview_value(
    request: &Value,
    context_manifest: Option<Value>,
) -> Result<ChatRequest, String> {
    let object = request
        .as_object()
        .ok_or_else(|| "LLM 预览 request 必须是对象".to_string())?;
    let messages_value = object
        .get("messages")
        .ok_or_else(|| "LLM 预览 request 缺少 messages".to_string())?
        .clone();
    let messages: Vec<ChatMessage> =
        serde_json::from_value(messages_value).map_err(|err| err.to_string())?;
    let temperature = object
        .get("temperature")
        .and_then(Value::as_f64)
        .map(|value| value as f32);
    let max_tokens = optional_usize_field(object.get("max_tokens"), "max_tokens")?;
    let max_tokens_profile = object
        .get("max_tokens_profile")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let response_format = match object.get("response_format") {
        Some(Value::Null) | None => None,
        Some(value) => Some(
            serde_json::from_value::<ResponseFormat>(value.clone())
                .map_err(|err| format!("response_format 解析失败: {err}"))?,
        ),
    };
    let tools = match object.get("tools") {
        Some(Value::Null) | None => None,
        Some(Value::Array(values)) => Some(values.clone()),
        Some(_) => return Err("tools 必须是数组".to_string()),
    };
    let tool_choice = object
        .get("tool_choice")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let stop_sequences = match object.get("stop_sequences") {
        Some(Value::Null) | None => None,
        Some(value) => Some(
            serde_json::from_value::<Vec<String>>(value.clone())
                .map_err(|err| format!("stop_sequences 解析失败: {err}"))?,
        ),
    };
    let mut chat_request = ChatRequest::new(messages, temperature, max_tokens, max_tokens_profile);
    chat_request.response_format = response_format;
    chat_request.tools = tools;
    chat_request.tool_choice = tool_choice;
    chat_request.stop_sequences = stop_sequences;
    chat_request.context_manifest_json = context_manifest;
    Ok(chat_request)
}

fn optional_usize_field(value: Option<&Value>, field: &str) -> Result<Option<usize>, String> {
    match value {
        Some(Value::Number(number)) => number
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .map(Some)
            .ok_or_else(|| format!("{field} 必须是非负整数")),
        Some(Value::Null) | None => Ok(None),
        Some(_) => Err(format!("{field} 必须是非负整数")),
    }
}
