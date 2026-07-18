use a4n_core::error::AppError;
use serde_json::{Map, Value};

pub(in crate::blueprint::json) fn reject_keys(
    object: &Map<String, Value>,
    keys: &[&str],
    context: &str,
) -> Result<(), AppError> {
    for key in keys {
        if object.contains_key(*key) {
            return Err(AppError::InvalidArgument(format!(
                "{context} 包含旧字段 {key}，请使用当前蓝图 JSON schema"
            )));
        }
    }
    Ok(())
}

pub(in crate::blueprint::json) fn reject_unknown_keys(
    object: &Map<String, Value>,
    allowed: &[&str],
    context: &str,
) -> Result<(), AppError> {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "{context} 包含未知字段 {key}；允许字段：{}",
                allowed.join(", ")
            )));
        }
    }
    Ok(())
}
