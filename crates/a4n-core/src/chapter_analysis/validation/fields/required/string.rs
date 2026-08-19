use serde_json::{Map, Value};

use super::super::super::analysis_schema_error;

pub(in crate::chapter_analysis::validation) fn required_string<'a>(
    root: &'a Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<&'a str, crate::error::AppError> {
    root.get(key)
        .ok_or_else(|| analysis_schema_error(format!("{context} 缺少必需字段 {key}")))?
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| analysis_schema_error(format!("{context}.{key} 必须是非空字符串")))
}
