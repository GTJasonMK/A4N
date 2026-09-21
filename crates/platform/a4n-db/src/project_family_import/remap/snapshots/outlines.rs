use a4n_db_models::models::project::source_snapshot_outline;
use serde_json::Value;

use super::super::json_ids::{json_object, remap_object_id, remap_required_object_id};
use crate::project_family_import::{
    ImportIdMaps, ProjectFamilyImportError, hash_json, map_required,
};

pub(in crate::project_family_import) fn remap_source_snapshot_outline_payload(
    outline: &source_snapshot_outline::Model,
    maps: &ImportIdMaps,
) -> Result<(Value, String), ProjectFamilyImportError> {
    let mut object = json_object(&outline.payload_json, "源快照大纲 payload_json")?;
    match outline.outline_kind.as_str() {
        "part" => {
            remap_required_object_id(
                &mut object,
                "id",
                &maps.part_outline_ids,
                "源快照大纲 payload_json",
            )?;
            remap_required_object_id(
                &mut object,
                "project_id",
                &maps.project_ids,
                "源快照大纲 payload_json",
            )?;
        }
        "chapter" => {
            remap_required_object_id(
                &mut object,
                "id",
                &maps.chapter_outline_ids,
                "源快照大纲 payload_json",
            )?;
            remap_required_object_id(
                &mut object,
                "project_id",
                &maps.project_ids,
                "源快照大纲 payload_json",
            )?;
            remap_object_id(
                &mut object,
                "part_id",
                &maps.part_outline_ids,
                "源快照大纲 payload_json",
            )?;
        }
        other => {
            return Err(ProjectFamilyImportError::invalid_package(format!(
                "源快照大纲类型无效: {other}"
            )));
        }
    }
    let payload = Value::Object(object);
    let payload_hash = hash_json(&payload);
    Ok((payload, payload_hash))
}

pub(in crate::project_family_import) fn remap_snapshot_outline_source_id(
    outline: &source_snapshot_outline::Model,
    maps: &ImportIdMaps,
) -> Result<i64, ProjectFamilyImportError> {
    match outline.outline_kind.as_str() {
        "part" => map_required(
            &maps.part_outline_ids,
            outline.source_outline_id,
            "源快照分部大纲 source_outline_id",
        ),
        "chapter" => map_required(
            &maps.chapter_outline_ids,
            outline.source_outline_id,
            "源快照章节大纲 source_outline_id",
        ),
        other => Err(ProjectFamilyImportError::invalid_package(format!(
            "源快照大纲类型无效: {other}"
        ))),
    }
}
