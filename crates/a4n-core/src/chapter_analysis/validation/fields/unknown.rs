use serde_json::{Map, Value};

use super::super::analysis_schema_error;

pub(in crate::chapter_analysis::validation) fn reject_unknown_fields(
    object: &Map<String, Value>,
    allowed: &[&str],
    context: &str,
) -> Result<(), crate::error::AppError> {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(analysis_schema_error(format!(
                "{context} 包含未知字段 {key}；允许字段：{}",
                allowed.join(", ")
            )));
        }
    }
    Ok(())
}
