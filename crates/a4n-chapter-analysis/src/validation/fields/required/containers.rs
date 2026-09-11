use serde_json::{Map, Value};

use super::super::super::analysis_schema_error;

pub(in crate::validation) fn required_object<'a>(
    root: &'a Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<&'a Map<String, Value>, a4n_error::AppError> {
    root.get(key)
        .ok_or_else(|| analysis_schema_error(format!("{context} 缺少必需字段 {key}")))?
        .as_object()
        .ok_or_else(|| analysis_schema_error(format!("{context}.{key} 必须是对象")))
}

pub(in crate::validation) fn required_array<'a>(
    root: &'a Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<&'a Vec<Value>, a4n_error::AppError> {
    root.get(key)
        .ok_or_else(|| analysis_schema_error(format!("{context} 缺少必需字段 {key}")))?
        .as_array()
        .ok_or_else(|| analysis_schema_error(format!("{context}.{key} 必须是数组")))
}
