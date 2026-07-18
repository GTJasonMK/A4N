use a4n_core::error::AppError;
use serde_json::{Map, Value};

use super::analysis_materialize_error;

pub fn reject_materialize_unknown_fields(
    object: &Map<String, Value>,
    allowed: &[&str],
    path: &str,
) -> Result<(), AppError> {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(analysis_materialize_error(format!(
                "{path} 包含未知字段 {key}；允许字段：{}",
                allowed.join(", ")
            )));
        }
    }
    Ok(())
}
