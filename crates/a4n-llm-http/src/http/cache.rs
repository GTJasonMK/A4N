use a4n_error::AppError;
use a4n_llm::provider::ChatRequest;
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
