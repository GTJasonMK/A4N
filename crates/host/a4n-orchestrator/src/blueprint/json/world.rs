use a4n_error::AppError;
use serde_json::Value;

mod arrays;
mod contract;
use arrays::{validate_object_array, validate_string_array};
use contract::validate_world_top_level_contract;

pub(super) fn validate_world_setting(value: &Value) -> Result<(), AppError> {
    validate_world_setting_with_policy(value, false)
}

pub(in crate::blueprint::json) fn validate_sparse_world_setting(
    value: &Value,
) -> Result<(), AppError> {
    validate_world_setting_with_policy(value, true)
}

fn validate_world_setting_with_policy(value: &Value, allow_sparse: bool) -> Result<(), AppError> {
    validate_world_top_level_contract(value, allow_sparse)?;

    validate_object_array(
        value,
        "key_locations",
        &["name", "description", "story_function"],
    )?;
    validate_object_array(
        value,
        "factions",
        &["name", "description", "agenda", "representative_characters"],
    )?;
    validate_object_array(value, "history_timeline", &["period", "event", "impact"])?;
    validate_object_array(
        value,
        "conflict_sources",
        &["name", "description", "stakes"],
    )?;
    validate_object_array(
        value,
        "mysteries_and_foreshadowing",
        &["mystery", "early_clues", "payoff_direction"],
    )?;
    validate_string_array(value, "genre_promises")?;

    Ok(())
}
