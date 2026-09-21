use a4n_error::AppError;
use serde_json::Value;

use super::analysis_materialize_error;

pub(in crate::chapter_analysis::owner) fn validate_required_string_enum(
    value: Option<&Value>,
    allowed: &[&str],
    path: &str,
) -> Result<(), AppError> {
    let text = value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| analysis_materialize_error(format!("{path} 必须是非空字符串")))?;
    if allowed.contains(&text) {
        Ok(())
    } else {
        Err(analysis_materialize_error(format!(
            "{path} 无效：{text}；允许值：{}",
            allowed.join("/")
        )))
    }
}
