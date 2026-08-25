use super::super::entities::{parse_characters, parse_relationships};
use super::super::fields::{required_array, required_object_clone, required_string};
use super::super::world::validate_world_setting;
use a4n_core::error::AppError;
use serde_json::Value;

pub(in crate::blueprint) fn validate_story_core_stage(value: &Value) -> Result<(), AppError> {
    for key in [
        "title",
        "target_audience",
        "genre",
        "style",
        "tone",
        "one_sentence_summary",
        "full_synopsis",
    ] {
        required_string(value, key, "blueprint")?;
    }
    Ok(())
}

pub(in crate::blueprint) fn validate_world_stage(value: &Value) -> Result<(), AppError> {
    let world_setting = required_object_clone(value, "world_setting", "blueprint")?;
    validate_world_setting(&world_setting)
}

pub(in crate::blueprint) fn validate_characters_stage(value: &Value) -> Result<(), AppError> {
    parse_characters(required_array(value, "characters", "blueprint")?)?;
    Ok(())
}

pub(in crate::blueprint) fn validate_relationships_stage(value: &Value) -> Result<(), AppError> {
    parse_relationships(required_array(value, "relationships", "blueprint")?)?;
    Ok(())
}
