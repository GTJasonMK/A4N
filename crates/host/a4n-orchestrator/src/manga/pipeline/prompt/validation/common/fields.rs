use a4n_error::AppError;
use serde_json::Value;

pub(in crate::manga::pipeline::prompt) fn required_manga_string(
    value: &Value,
    key: &str,
    path: &str,
) -> Result<String, AppError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path}.{key} 必须是非空字符串")))
}
