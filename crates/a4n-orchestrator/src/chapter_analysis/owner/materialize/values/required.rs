use a4n_error::AppError;
use serde_json::{Map, Value};

use super::analysis_materialize_error;

pub fn required_materialize_string(
    object: &Map<String, Value>,
    key: &str,
    path: &str,
) -> Result<String, AppError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| analysis_materialize_error(format!("{path} 必须是非空字符串")))
}
