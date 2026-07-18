use a4n_core::error::AppError;
use serde_json::Value;

use super::super::fields::{
    reject_unknown_keys, required_array, required_non_empty_value, required_object_with_keys,
};

const WORLD_SETTING_FIELDS: &[&str] = &[
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
];

const NON_EMPTY_ARRAY_FIELDS: &[&str] = &[
    "key_locations",
    "factions",
    "history_timeline",
    "conflict_sources",
    "mysteries_and_foreshadowing",
    "genre_promises",
];

pub(super) fn validate_world_top_level_contract(value: &Value) -> Result<(), AppError> {
    let object = value.as_object().ok_or_else(|| {
        AppError::InvalidArgument("blueprint.world_setting 必须是 JSON 对象".to_string())
    })?;
    reject_unknown_keys(object, WORLD_SETTING_FIELDS, "blueprint.world_setting")?;
    for key in WORLD_SETTING_FIELDS {
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
    for key in NON_EMPTY_ARRAY_FIELDS {
        if required_array(value, key, "blueprint.world_setting")?.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "blueprint.world_setting.{key} 不能为空数组"
            )));
        }
    }
    Ok(())
}
