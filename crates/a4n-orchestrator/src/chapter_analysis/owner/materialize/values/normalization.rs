use a4n_core::error::AppError;
use serde_json::Value;

use super::analysis_materialize_error;

mod array;
pub(in crate::chapter_analysis::owner) use array::normalize_string_array;

pub(in crate::chapter_analysis::owner) fn normalize_optional_string(
    value: Option<&mut Value>,
    path: &str,
) -> Result<(), AppError> {
    match value {
        Some(Value::Null) => Ok(()),
        Some(Value::String(text)) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return Err(analysis_materialize_error(format!(
                    "{path} 必须是非空字符串或 null"
                )));
            }
            *text = trimmed.to_string();
            Ok(())
        }
        Some(_) => Err(analysis_materialize_error(format!(
            "{path} 必须是字符串或 null"
        ))),
        None => Err(analysis_materialize_error(format!("{path} 缺失"))),
    }
}
