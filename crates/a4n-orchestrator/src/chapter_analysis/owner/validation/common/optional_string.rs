use a4n_core::error::AppError;
use serde_json::Value;

use super::super::super::materialize::analysis_materialize_error;

pub fn validate_optional_stage_string(value: Option<&Value>, path: &str) -> Result<(), AppError> {
    match value {
        Some(Value::Null) => Ok(()),
        Some(Value::String(text)) if !text.trim().is_empty() => Ok(()),
        Some(Value::String(_)) => Err(analysis_materialize_error(format!(
            "{path} 必须是非空字符串或 null"
        ))),
        Some(_) => Err(analysis_materialize_error(format!(
            "{path} 必须是字符串或 null"
        ))),
        None => Err(analysis_materialize_error(format!("{path} 缺失"))),
    }
}
