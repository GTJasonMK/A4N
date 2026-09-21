use serde_json::Value;

use crate::blueprint::validate_sparse_blueprint_sections;

pub(in crate::import_reverse::application::apply) fn validate_reverse_blueprint_sections(
    data: &Value,
    world_setting: &Value,
) -> Result<(), String> {
    let characters = data
        .get("characters")
        .and_then(Value::as_array)
        .ok_or_else(|| "reverse_blueprint.characters 必须是数组".to_string())?;
    let relationships = data
        .get("relationships")
        .and_then(Value::as_array)
        .ok_or_else(|| "reverse_blueprint.relationships 必须是数组".to_string())?;
    validate_sparse_blueprint_sections(world_setting, characters, relationships)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;
