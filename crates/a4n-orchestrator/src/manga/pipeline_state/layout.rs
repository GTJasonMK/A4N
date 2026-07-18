use a4n_core::error::AppError;
use serde_json::{Map, Value};

use super::contracts::PipelineProgress;

pub(crate) fn pipeline_status_from_layout<'a>(
    layout: Option<&'a Value>,
    context: &str,
) -> Result<&'a str, AppError> {
    let layout = layout_object(layout, context)?;
    let status = layout
        .get("pipeline_status")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|status| !status.is_empty())
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("{context}.pipeline_status 必须是非空字符串"))
        })?;
    Ok(status)
}

pub(crate) fn pipeline_progress_from_layout(
    layout: Option<&Value>,
    context: &str,
) -> Result<PipelineProgress, AppError> {
    let layout = layout_object(layout, context)?;
    let current = required_nonnegative_i32(layout, "current", context)?;
    let total = required_positive_i32(layout, "total", context)?;
    if current > total {
        return Err(AppError::InvalidArgument(format!(
            "{context}.current 不能大于 {context}.total"
        )));
    }
    Ok(PipelineProgress { current, total })
}

fn layout_object<'a>(
    layout: Option<&'a Value>,
    context: &str,
) -> Result<&'a Map<String, Value>, AppError> {
    layout
        .and_then(Value::as_object)
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.layout 必须是 JSON 对象")))
}

fn required_nonnegative_i32(
    layout: &Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<i32, AppError> {
    let value = required_i32(layout, key, context)?;
    if value < 0 {
        return Err(AppError::InvalidArgument(format!(
            "{context}.{key} 必须是非负整数"
        )));
    }
    Ok(value)
}

fn required_positive_i32(
    layout: &Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<i32, AppError> {
    let value = required_i32(layout, key, context)?;
    if value <= 0 {
        return Err(AppError::InvalidArgument(format!(
            "{context}.{key} 必须是正整数"
        )));
    }
    Ok(value)
}

fn required_i32(layout: &Map<String, Value>, key: &str, context: &str) -> Result<i32, AppError> {
    let raw = layout
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.{key} 必须是整数")))?;
    i32::try_from(raw)
        .map_err(|_| AppError::InvalidArgument(format!("{context}.{key} 超出 i32 可表示范围")))
}
