use a4n_error::AppError;
use a4n_llm_provider_contract::ChatRequest;
use serde_json::{Map, Value, json};

pub(super) fn openai_text_content(
    request: &ChatRequest,
    index: usize,
    kind: &str,
) -> Result<Value, AppError> {
    let parts = request.cached_text_parts(index)?;
    if !parts.iter().any(|(_, cached)| *cached) {
        return Ok(json!(request.messages[index].content));
    }
    Ok(Value::Array(
        parts
            .into_iter()
            .map(|(text, cached)| {
                let mut part = json!({"type": kind, "text": text});
                if cached {
                    part["prompt_cache_breakpoint"] = json!({"mode": "explicit"});
                }
                part
            })
            .collect(),
    ))
}

pub(super) fn openai_content_with_images(
    request: &ChatRequest,
    index: usize,
    text_kind: &str,
    image_kind: &str,
) -> Result<Value, AppError> {
    let text = openai_text_content(request, index, text_kind)?;
    let is_last_user =
        matches!(
            request.messages[index].role,
            a4n_llm_provider_contract::MessageRole::User
        ) && request.messages.iter().rposition(|message| {
            matches!(message.role, a4n_llm_provider_contract::MessageRole::User)
        }) == Some(index);
    if request.vision_images.is_empty() || !is_last_user {
        return Ok(text);
    }
    let mut parts = match text {
        Value::Array(parts) => parts,
        Value::String(text) => vec![json!({"type": text_kind, "text": text})],
        other => {
            return Err(AppError::InvalidArgument(format!(
                "视觉 LLM 文本内容不是字符串或数组: {other}"
            )));
        }
    };
    for image in &request.vision_images {
        if !image.media_type.starts_with("image/") || image.data_base64.is_empty() {
            return Err(AppError::InvalidArgument("视觉输入图片数据无效".into()));
        }
        let data_uri = format!("data:{};base64,{}", image.media_type, image.data_base64);
        parts.push(match image_kind {
            "image_url" => json!({
                "type": "image_url",
                "image_url": {"url": data_uri}
            }),
            "input_image" => json!({
                "type": "input_image",
                "image_url": data_uri
            }),
            other => {
                return Err(AppError::InvalidArgument(format!(
                    "不支持的视觉输入协议块: {other}"
                )));
            }
        });
    }
    Ok(Value::Array(parts))
}

pub(super) fn apply_openai_cache_options(
    request: &ChatRequest,
    body: &mut Map<String, Value>,
) -> Result<(), AppError> {
    if let Some(key) = &request.cache_scope_key {
        body.insert("prompt_cache_key".into(), json!(key));
    }
    if let Some(mode) = request.cache_mode {
        let options = body
            .entry("prompt_cache_options")
            .or_insert_with(|| json!({}));
        let options = options
            .as_object_mut()
            .ok_or_else(|| AppError::ConfigError("prompt_cache_options 必须是对象".into()))?;
        options.insert("mode".into(), json!(mode));
    }
    Ok(())
}
