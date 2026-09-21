use serde_json::Value;

use crate::canvas::CanvasDbError;

pub fn validate_canvas_record_viewport_json(
    value: Option<&Value>,
    label: &str,
) -> Result<(), CanvasDbError> {
    if let Some(value) = value
        && !value.is_object()
    {
        return Err(CanvasDbError::InvalidArgument(format!(
            "{label} 必须是 JSON 对象"
        )));
    }
    Ok(())
}

pub(in crate::canvas::graph_record) fn required_canvas_record_text(
    value: &str,
    field: &str,
) -> Result<String, CanvasDbError> {
    let value = value.trim();
    if value.is_empty() {
        Err(CanvasDbError::InvalidArgument(format!("{field} 不能为空")))
    } else {
        Ok(value.to_string())
    }
}
