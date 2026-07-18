use a4n_core::error::AppError;
use serde_json::Value;

const GALGAME_CG_IMPORTANCE: &[&str] = &["normal", "high", "critical"];

#[derive(Debug)]
pub(in crate::galgame) struct GalgameEventCgDirective {
    pub(in crate::galgame) note: String,
    pub(in crate::galgame) importance: String,
}

pub(in crate::galgame) fn event_cg_directive(
    value: &Value,
    line_id: i64,
) -> Result<Option<GalgameEventCgDirective>, AppError> {
    event_cg_directive_for_label(value, &format!("Galgame 行 {line_id}"))
}

fn event_cg_directive_for_label(
    value: &Value,
    line_label: &str,
) -> Result<Option<GalgameEventCgDirective>, AppError> {
    let Some(object) = value.as_object() else {
        return Err(AppError::InvalidArgument(format!(
            "{line_label} presentation_json 必须是 JSON 对象"
        )));
    };
    let cg_enabled = match object.get("cg") {
        Some(Value::Bool(enabled)) => *enabled,
        Some(Value::Null) | None => false,
        Some(_) => {
            return Err(AppError::InvalidArgument(format!(
                "{line_label} presentation_json.cg 必须是布尔值"
            )));
        }
    };
    let note = event_cg_note_text(object);
    let importance = match object.get("cg_importance") {
        Some(Value::Null) | None => "normal".to_string(),
        Some(Value::String(value)) => {
            let value = value.trim();
            if value.is_empty() {
                "normal".to_string()
            } else if GALGAME_CG_IMPORTANCE.contains(&value) {
                value.to_string()
            } else {
                return Err(AppError::InvalidArgument(format!(
                    "{line_label} presentation_json.cg_importance 无效: {value}"
                )));
            }
        }
        Some(_) => {
            return Err(AppError::InvalidArgument(format!(
                "{line_label} presentation_json.cg_importance 必须是字符串"
            )));
        }
    };
    if !cg_enabled && note.is_none() {
        return Ok(None);
    }
    let note = note.ok_or_else(|| {
        AppError::InvalidArgument(format!("{line_label} 启用事件 CG 时必须提供 cg_note"))
    })?;
    Ok(Some(GalgameEventCgDirective { note, importance }))
}

fn event_cg_note_text(object: &serde_json::Map<String, Value>) -> Option<String> {
    for key in ["cg_note", "cg_prompt", "event_cg_note"] {
        if let Some(note) = object.get(key).and_then(Value::as_str) {
            let note = note.trim();
            if !note.is_empty() {
                return Some(note.to_string());
            }
        }
    }
    None
}

pub(crate) fn validate_presentation_json(value: &Value, line_label: &str) -> Result<(), AppError> {
    event_cg_directive_for_label(value, line_label).map(|_| ())
}
