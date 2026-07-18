use a4n_core::error::AppError;
use serde_json::Value;

use super::super::super::materialize::analysis_materialize_error;

pub(super) fn validate_required_fields(value: &Value) -> Result<(), AppError> {
    let root = value
        .as_object()
        .ok_or_else(|| analysis_materialize_error("chapter_foreshadowing_events 必须是对象"))?;
    if !root.contains_key("foreshadowing") {
        return Err(analysis_materialize_error(
            "chapter_foreshadowing_events 缺少 foreshadowing",
        ));
    }
    if !root.contains_key("key_events") {
        return Err(analysis_materialize_error(
            "chapter_foreshadowing_events 缺少 key_events",
        ));
    }
    Ok(())
}
