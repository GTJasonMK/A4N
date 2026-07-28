use a4n_core::error::AppError;
use serde_json::Value;

pub fn parse_outline_json_object(content: &str, prompt_name: &str) -> Result<Value, AppError> {
    let trimmed = content.trim();
    let value: Value = serde_json::from_str(trimmed).map_err(|error| {
        AppError::InvalidArgument(format!(
            "模型返回无法解析为 {prompt_name} JSON: {error}；输出预览：{}",
            json_error_preview(trimmed)
        ))
    })?;
    if !value.is_object() {
        return Err(AppError::InvalidArgument(format!(
            "{prompt_name} 结果必须是 JSON 对象"
        )));
    }
    Ok(value)
}

fn json_error_preview(content: &str) -> String {
    if content.is_empty() {
        return "<empty>".to_string();
    }
    content.chars().take(240).collect()
}

pub fn reject_keys(json: &Value, keys: &[&str], context: &str) -> Result<(), AppError> {
    for key in keys {
        if json.get(*key).is_some() {
            return Err(AppError::InvalidArgument(format!(
                "{context} 不再接受字段 {key}"
            )));
        }
    }
    Ok(())
}

pub fn reject_unknown_keys(json: &Value, allowed: &[&str], context: &str) -> Result<(), AppError> {
    let object = json
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context} 必须是 JSON 对象")))?;
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "{context} 包含未知字段 {key}"
            )));
        }
    }
    Ok(())
}
