use super::common::{remap_nested_object_ids, required_mapped_target_entity_id};
use crate::project::family::import::{ImportIdMaps, ProjectFamilyImportError, map_required};
use a4n_db_models::models::generation::generation_spec;
use serde_json::Value;

pub(in crate::project::family::import::remap::generation) fn remap_galgame_background_params(
    spec: &generation_spec::Model,
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let project_id = map_required(
        &maps.project_ids,
        spec.project_id,
        "Galgame 背景 canonical_key project_id",
    )?;
    let scene_id = required_mapped_target_entity_id(
        spec,
        &maps.galgame_scene_ids,
        "Galgame 背景 target_entity_id",
    )?;
    object.insert(
        "canonical_key".to_string(),
        Value::String(format!("galgame:{project_id}:background:scene:{scene_id}")),
    );
    remap_nested_object_ids(
        object,
        "scene",
        "Galgame 背景 params_json.scene",
        &[("id", &maps.galgame_scene_ids)],
    )?;
    remap_nested_object_ids(
        object,
        "source",
        "Galgame 背景 params_json.source",
        &[
            (
                "source_snapshot_chapter_id",
                &maps.source_snapshot_chapter_ids,
            ),
            ("source_chapter_id", &maps.chapter_ids),
        ],
    )
}
