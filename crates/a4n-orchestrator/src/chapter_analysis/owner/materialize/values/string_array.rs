use a4n_error::AppError;
use serde_json::Value;

use super::analysis_materialize_error;

pub fn validate_string_array(value: Option<&Value>, path: &str) -> Result<(), AppError> {
    let values = value
        .and_then(Value::as_array)
        .ok_or_else(|| analysis_materialize_error(format!("{path} 必须是数组")))?;
    for (index, value) in values.iter().enumerate() {
        let text = value
            .as_str()
            .map(str::trim)
            .ok_or_else(|| analysis_materialize_error(format!("{path}[{index}] 必须是字符串")))?;
        if text.is_empty() {
            return Err(analysis_materialize_error(format!(
                "{path}[{index}] 不能为空"
            )));
        }
    }
    Ok(())
}
