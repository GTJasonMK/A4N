use a4n_error::AppError;
use serde_json::Value;

use super::super::analysis_materialize_error;

pub fn normalize_string_array(value: Option<&mut Value>, path: &str) -> Result<(), AppError> {
    let values = value
        .and_then(Value::as_array_mut)
        .ok_or_else(|| analysis_materialize_error(format!("{path} 必须是数组")))?;
    let mut normalized = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let text = value
            .as_str()
            .map(str::trim)
            .ok_or_else(|| analysis_materialize_error(format!("{path}[{index}] 必须是字符串")))?;
        if text.is_empty()
            || normalized
                .iter()
                .any(|item: &Value| item.as_str() == Some(text))
        {
            continue;
        }
        normalized.push(Value::String(text.to_string()));
    }
    *values = normalized;
    Ok(())
}
