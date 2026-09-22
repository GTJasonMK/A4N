use super::common::{remap_nested_object_ids, required_mapped_target_entity_id};
use crate::project::family::import::{ImportIdMaps, ProjectFamilyImportError, map_required};
use a4n_db_models::models::generation::generation_spec;
use serde_json::Value;

pub(in crate::project::family::import::remap::generation) fn remap_galgame_cg_params(
    spec: &generation_spec::Model,
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let project_id = map_required(
        &maps.project_ids,
        spec.project_id,
        "Galgame CG canonical_key project_id",
    )?;
    let line_id = required_mapped_target_entity_id(
        spec,
        &maps.galgame_line_ids,
        "Galgame CG target_entity_id",
    )?;
    object.insert(
        "canonical_key".to_string(),
        Value::String(format!("galgame:{project_id}:cg:line:{line_id}")),
    );
    remap_nested_object_ids(
        object,
        "line",
        "Galgame CG params_json.line",
        &[
            ("id", &maps.galgame_line_ids),
            ("speaker_profile_id", &maps.character_profile_ids),
        ],
    )?;
    remap_nested_object_ids(
        object,
        "scene",
        "Galgame CG params_json.scene",
        &[("id", &maps.galgame_scene_ids)],
    )
}
