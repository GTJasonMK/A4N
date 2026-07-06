use a4n_core::error::AppError;
use serde_json::Value;

use super::fields::{
    reject_unknown_keys, required_array, required_non_empty_value, required_object_with_keys,
};

pub(super) fn validate_world_setting(value: &Value) -> Result<(), AppError> {
    let object = value.as_object().ok_or_else(|| {
        AppError::InvalidArgument("blueprint.world_setting 必须是 JSON 对象".to_string())
    })?;
    reject_unknown_keys(
        object,
        &[
            "premise",
            "core_rules",
            "technology_or_magic",
            "key_locations",
            "factions",
            "history_timeline",
            "culture_and_society",
            "conflict_sources",
            "mysteries_and_foreshadowing",
            "genre_promises",
        ],
        "blueprint.world_setting",
    )?;
    for key in [
        "premise",
        "core_rules",
        "technology_or_magic",
        "key_locations",
        "factions",
        "history_timeline",
        "culture_and_society",
        "conflict_sources",
        "mysteries_and_foreshadowing",
        "genre_promises",
    ] {
        required_non_empty_value(value, key, "blueprint.world_setting")?;
    }

    required_object_with_keys(
        value,
        "technology_or_magic",
        "blueprint.world_setting",
        &["source", "rules", "limits", "costs"],
    )?;
    required_object_with_keys(
        value,
        "culture_and_society",
        "blueprint.world_setting",
        &["values", "class_structure", "daily_life"],
    )?;

    for key in [
        "key_locations",
        "factions",
        "history_timeline",
        "conflict_sources",
        "mysteries_and_foreshadowing",
        "genre_promises",
    ] {
        let items = required_array(value, key, "blueprint.world_setting")?;
        if items.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "blueprint.world_setting.{key} 不能为空数组"
            )));
        }
    }

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

fn validate_object_array(value: &Value, key: &str, allowed: &[&str]) -> Result<(), AppError> {
    let items = required_array(value, key, "blueprint.world_setting")?;
    for (index, item) in items.iter().enumerate() {
        let context = format!("blueprint.world_setting.{key}[{index}]");
        let object = item
            .as_object()
            .ok_or_else(|| AppError::InvalidArgument(format!("{context} 必须是 JSON 对象")))?;
        reject_unknown_keys(object, allowed, &context)?;
        for required_key in allowed {
            required_non_empty_value(item, required_key, &context)?;
        }
    }
    Ok(())
}

fn validate_string_array(value: &Value, key: &str) -> Result<(), AppError> {
    let items = required_array(value, key, "blueprint.world_setting")?;
    for (index, item) in items.iter().enumerate() {
        let text = item.as_str().map(str::trim).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "blueprint.world_setting.{key}[{index}] 必须是非空字符串"
            ))
        })?;
        if text.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "blueprint.world_setting.{key}[{index}] 必须是非空字符串"
            )));
        }
    }
    Ok(())
}
