use super::super::super::json_ids::remap_object_id;
use super::common::{remap_object_array_ids, required_mapped_target_entity_id};
use crate::project_family_import::{ImportIdMaps, ProjectFamilyImportError, map_required};
use a4n_core::models::generation::generation_spec;
use serde_json::Value;

pub(in crate::project_family_import::remap::generation) fn remap_galgame_portrait_params(
    spec: &generation_spec::Model,
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let project_id = map_required(
        &maps.project_ids,
        spec.project_id,
        "Galgame 立绘差分 canonical_key project_id",
    )?;
    let profile_id = required_mapped_target_entity_id(
        spec,
        &maps.character_profile_ids,
        "Galgame 立绘差分 target_entity_id",
    )?;
    remap_object_id(
        object,
        "character_profile_id",
        &maps.character_profile_ids,
        "Galgame 立绘差分 params_json",
    )?;
    remap_object_array_ids(
        object,
        "source_line_ids",
        &maps.galgame_line_ids,
        "Galgame 立绘差分 params_json.source_line_ids",
    )?;
    let variant_key = galgame_portrait_variant_key(object)?;
    object.insert(
        "canonical_key".to_string(),
        Value::String(format!(
            "galgame:{project_id}:portrait:{profile_id}:{variant_key}"
        )),
    );
    Ok(())
}

fn galgame_portrait_variant_key(
    object: &serde_json::Map<String, Value>,
) -> Result<String, ProjectFamilyImportError> {
    let expression = object
        .get("expression")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(
                "Galgame 立绘差分 params_json 缺少 expression",
            )
        })?;
    let pose = object
        .get("pose")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ProjectFamilyImportError::invalid_package("Galgame 立绘差分 params_json 缺少 pose")
        })?;
    Ok(format!("expression:{expression}:pose:{pose}"))
}
