use a4n_core::error::AppError;
use serde_json::Value;

use super::rules::require_object_value;

pub fn parse_galgame_json_object(raw: &str, context: &str) -> Result<Value, AppError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(AppError::InvalidArgument(format!("{context} 返回空内容")));
    }
    if raw.starts_with("```") {
        return Err(AppError::InvalidArgument(format!(
            "{context} 不能用 Markdown 代码围栏包裹 JSON"
        )));
    }
    let value: Value = serde_json::from_str(raw).map_err(|err| {
        let reason = if err.is_eof() {
            "不完整 JSON"
        } else {
            "JSON 解析失败"
        };
        AppError::InvalidArgument(format!("{context} {reason}: {err}"))
    })?;
    require_object_value(value, context)
}
