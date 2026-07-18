use a4n_core::models::galgame::galgame_line;
use serde_json::{Map, Value};

use super::super::super::GALGAME_CG_IMPORTANCE;

pub(super) fn validate_galgame_line_presentation_json(
    line: &galgame_line::Model,
) -> Result<(), String> {
    let object = line.presentation_json.as_object().ok_or_else(|| {
        format!(
            "项目族导入包 Galgame 行 presentation_json 必须是 JSON 对象: line_id={}",
            line.id
        )
    })?;
    let cg_enabled = match object.get("cg") {
        Some(Value::Bool(value)) => *value,
        Some(Value::Null) | None => false,
        Some(_) => {
            return Err(format!(
                "项目族导入包 Galgame 行 presentation_json.cg 必须是布尔值: line_id={}",
                line.id
            ));
        }
    };
    if let Some(value) = object.get("cg_importance")
        && !value.is_null()
    {
        let importance = value.as_str().map(str::trim).ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 行 presentation_json.cg_importance 必须是字符串: line_id={}",
                line.id
            )
        })?;
        if !importance.is_empty() {
            ensure_package!(
                GALGAME_CG_IMPORTANCE.contains(&importance),
                "项目族导入包 Galgame 行 presentation_json.cg_importance 无效: line_id={}, cg_importance={}",
                line.id,
                importance
            );
        }
    }
    let note = galgame_event_cg_note_text(object);
    if cg_enabled || note.is_some() {
        ensure_package!(
            note.is_some(),
            "项目族导入包 Galgame 行启用事件 CG 时必须提供 cg_note: line_id={}",
            line.id
        );
    }
    Ok(())
}

fn galgame_event_cg_note_text(object: &Map<String, Value>) -> Option<&str> {
    for key in ["cg_note", "cg_prompt", "event_cg_note"] {
        if let Some(value) = object
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(value);
        }
    }
    None
}
