use serde_json::{Map, Value};

use super::super::analysis_schema_error;

pub fn required_string_or_null(
    root: &Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<(), crate::error::AppError> {
    match root.get(key) {
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(()),
        Some(Value::String(_)) => Err(analysis_schema_error(format!(
            "{context}.{key} 必须是非空字符串或 null"
        ))),
        Some(Value::Null) => Ok(()),
        Some(_) => Err(analysis_schema_error(format!(
            "{context}.{key} 必须是字符串或 null"
        ))),
        None => Err(analysis_schema_error(format!(
            "{context} 缺少必需字段 {key}"
        ))),
    }
}
