use super::super::super::{ImportIdMaps, ProjectFamilyImportError};
use super::super::{json_object, remap_object_ids, remap_profile_source_id};
use super::decisions::remap_source_sync_decision_payload;
use a4n_db_models::models::source_sync::source_sync_conflict;
use serde_json::Value;

pub(in crate::project::family::import) fn remap_conflict_source_summary(
    conflict: &source_sync_conflict::Model,
    maps: &ImportIdMaps,
) -> Result<Value, ProjectFamilyImportError> {
    let mut object = json_object(
        &conflict.source_summary_json,
        "同步冲突 source_summary_json",
    )?;
    match conflict.conflict_scope.as_str() {
        "profile" => remap_profile_source_id(
            &mut object,
            "profile_kind",
            "source_profile_id",
            maps,
            "同步冲突 source_summary_json",
        )?,
        "text_anchor" => {
            remap_object_ids(
                &mut object,
                "同步冲突 source_summary_json",
                &[
                    ("old_text_anchor_id", &maps.text_anchor_ids),
                    ("new_text_anchor_id", &maps.text_anchor_ids),
                ],
            )?;
        }
        _ => {}
    }
    Ok(Value::Object(object))
}

pub(in crate::project::family::import) fn remap_conflict_local_summary(
    conflict: &source_sync_conflict::Model,
    maps: &ImportIdMaps,
) -> Result<Value, ProjectFamilyImportError> {
    let mut object = json_object(&conflict.local_summary_json, "同步冲突 local_summary_json")?;
    remap_object_ids(
        &mut object,
        "同步冲突 local_summary_json",
        &[
            ("generation_spec_id", &maps.generation_spec_ids),
            ("galgame_scene_id", &maps.galgame_scene_ids),
            ("galgame_line_id", &maps.galgame_line_ids),
        ],
    )?;
    Ok(Value::Object(object))
}

pub(in crate::project::family::import) fn remap_conflict_affected_refs(
    conflict: &source_sync_conflict::Model,
    maps: &ImportIdMaps,
) -> Result<Value, ProjectFamilyImportError> {
    let mut object = json_object(&conflict.affected_refs_json, "同步冲突 affected_refs_json")?;
    remap_object_ids(
        &mut object,
        "同步冲突 affected_refs_json",
        &[
            ("generation_spec_id", &maps.generation_spec_ids),
            ("source_chapter_id", &maps.chapter_ids),
            (
                "source_snapshot_chapter_id",
                &maps.source_snapshot_chapter_ids,
            ),
            (
                "old_source_snapshot_chapter_id",
                &maps.source_snapshot_chapter_ids,
            ),
            ("old_text_anchor_id", &maps.text_anchor_ids),
            ("new_text_anchor_id", &maps.text_anchor_ids),
            ("galgame_scene_id", &maps.galgame_scene_ids),
            ("galgame_line_id", &maps.galgame_line_ids),
        ],
    )?;
    remap_profile_source_id(
        &mut object,
        "profile_kind",
        "source_profile_id",
        maps,
        "同步冲突 affected_refs_json",
    )?;
    Ok(Value::Object(object))
}

pub(in crate::project::family::import) fn remap_conflict_decision_json(
    conflict: &source_sync_conflict::Model,
    maps: &ImportIdMaps,
) -> Result<Option<Value>, ProjectFamilyImportError> {
    match (
        &conflict.decision_json,
        conflict.resolution_strategy.as_deref(),
    ) {
        (Some(value), Some(strategy)) => Ok(Some(remap_source_sync_decision_payload(
            strategy,
            value,
            maps,
            "同步冲突 decision_json",
        )?)),
        (Some(_), None) => Err(ProjectFamilyImportError::invalid_package(format!(
            "同步冲突 decision_json 缺少 resolution_strategy: {}",
            conflict.id
        ))),
        (None, _) => Ok(None),
    }
}
