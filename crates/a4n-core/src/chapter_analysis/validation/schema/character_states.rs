use serde_json::{Map, Value};

use super::super::analysis_schema_error;
use super::super::fields::{
    reject_unknown_fields, required_array, required_object, required_string_or_null,
};

pub(super) fn validate_character_states_schema(
    root: &Map<String, Value>,
) -> Result<(), crate::error::AppError> {
    let character_states = required_object(root, "character_states", "chapter_analysis")?;
    for (name, state) in character_states {
        if name.trim().is_empty() {
            return Err(analysis_schema_error(
                "chapter_analysis.character_states 角色名不能为空",
            ));
        }
        let state = state.as_object().ok_or_else(|| {
            analysis_schema_error(format!(
                "chapter_analysis.character_states.{name} 必须是对象"
            ))
        })?;
        reject_unknown_fields(
            state,
            &["location", "status", "changes", "emotional_state"],
            &format!("chapter_analysis.character_states.{name}"),
        )?;
        required_string_or_null(state, "location", "chapter_analysis.character_states")?;
        required_string_or_null(state, "status", "chapter_analysis.character_states")?;
        required_array(state, "changes", "chapter_analysis.character_states")?;
        required_string_or_null(
            state,
            "emotional_state",
            "chapter_analysis.character_states",
        )?;
    }
    Ok(())
}
