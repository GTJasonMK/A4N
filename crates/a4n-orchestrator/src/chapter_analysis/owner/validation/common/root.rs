use a4n_core::error::AppError;
use serde_json::Value;

use super::super::super::materialize::{
    analysis_materialize_error, reject_materialize_unknown_fields,
};

pub fn reject_root_unknown_fields(
    value: &Value,
    allowed: &[&str],
    path: &str,
) -> Result<(), AppError> {
    let object = value
        .as_object()
        .ok_or_else(|| analysis_materialize_error(format!("{path} 必须是 JSON 对象")))?;
    reject_materialize_unknown_fields(object, allowed, path)
}
