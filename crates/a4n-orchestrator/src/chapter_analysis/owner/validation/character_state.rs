use a4n_chapter_analysis::CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE;
use a4n_error::AppError;
use serde_json::Value;

use super::{super::materialize::analysis_materialize_error, common::reject_root_unknown_fields};

mod entry;
use entry::validate_character_state_entry;

pub fn validate_chapter_character_state(value: &Value) -> Result<(), AppError> {
    reject_root_unknown_fields(
        value,
        &["character_states"],
        CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE,
    )?;
    let states = value
        .get("character_states")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            analysis_materialize_error("chapter_character_state.character_states 必须是对象")
        })?;
    for (name, state) in states {
        if name.trim().is_empty() {
            return Err(analysis_materialize_error(
                "chapter_character_state.character_states 角色名不能为空",
            ));
        }
        validate_character_state_entry(name, state)?;
    }
    Ok(())
}
