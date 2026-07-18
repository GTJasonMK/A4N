use a4n_core::error::AppError;
use serde_json::Value;

use super::super::super::materialize::{
    analysis_materialize_error, reject_materialize_unknown_fields, validate_string_array,
};
use super::super::common::validate_optional_stage_string;

pub(super) fn validate_character_state_entry(name: &str, state: &Value) -> Result<(), AppError> {
    let object = state.as_object().ok_or_else(|| {
        analysis_materialize_error(format!(
            "chapter_character_state.character_states.{name} 必须是对象"
        ))
    })?;
    reject_materialize_unknown_fields(
        object,
        &["location", "status", "changes", "emotional_state"],
        &format!("chapter_character_state.character_states.{name}"),
    )?;
    validate_optional_stage_string(
        object.get("location"),
        &format!("chapter_character_state.character_states.{name}.location"),
    )?;
    validate_optional_stage_string(
        object.get("status"),
        &format!("chapter_character_state.character_states.{name}.status"),
    )?;
    validate_string_array(
        object.get("changes"),
        &format!("chapter_character_state.character_states.{name}.changes"),
    )?;
    validate_optional_stage_string(
        object.get("emotional_state"),
        &format!("chapter_character_state.character_states.{name}.emotional_state"),
    )
}
