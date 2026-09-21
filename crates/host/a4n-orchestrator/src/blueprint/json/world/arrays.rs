use super::super::fields::{reject_unknown_keys, required_array, required_non_empty_value};
use a4n_error::AppError;
use serde_json::Value;

pub(super) fn validate_object_array(
    value: &Value,
    key: &str,
    allowed: &[&str],
) -> Result<(), AppError> {
    let items = required_array(value, key, "blueprint.world_setting")?;
    for (index, item) in items.iter().enumerate() {
        let context = format!("blueprint.world_setting.{key}[{index}]");
        let object = item
            .as_object()
            .ok_or_else(|| AppError::InvalidArgument(format!("{context} 必须是 JSON 对象")))?;
        reject_unknown_keys(object, allowed, &context)?;
        for required_key in allowed {
            required_non_empty_value(item, required_key, &context)?;
        }
    }
    Ok(())
}

pub(super) fn validate_string_array(value: &Value, key: &str) -> Result<(), AppError> {
    let items = required_array(value, key, "blueprint.world_setting")?;
    for (index, item) in items.iter().enumerate() {
        let text = item.as_str().map(str::trim).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "blueprint.world_setting.{key}[{index}] 必须是非空字符串"
            ))
        })?;
        if text.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "blueprint.world_setting.{key}[{index}] 必须是非空字符串"
            )));
        }
    }
    Ok(())
}
