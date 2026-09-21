use serde_json::Value;

pub(in crate::actions::request_preview) fn prompt_from_request(request: &Value) -> Option<String> {
    request
        .get("prompt")
        .or_else(|| request.pointer("/fields/prompt"))
        .or_else(|| request.pointer("/parameters/prompt"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .or_else(|| prompt_from_messages(request.get("messages")?))
}

fn prompt_from_messages(messages: &Value) -> Option<String> {
    messages.as_array()?.iter().rev().find_map(|message| {
        let role = message.get("role").and_then(Value::as_str).unwrap_or("");
        if role != "user" {
            return None;
        }
        message_content_text(message.get("content")?)
    })
}

fn message_content_text(content: &Value) -> Option<String> {
    if let Some(text) = content
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        return Some(text.to_string());
    }
    let parts = content.as_array()?;
    let text = parts
        .iter()
        .filter_map(|part| {
            part.get("text")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    if text.is_empty() { None } else { Some(text) }
}
