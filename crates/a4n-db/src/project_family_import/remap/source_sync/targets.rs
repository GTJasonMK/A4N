use super::super::super::{ImportIdMaps, ProjectFamilyImportError, map_required};
use a4n_core::models::source_sync::source_sync_conflict;

pub(in crate::project_family_import) fn remap_source_sync_target_id(
    conflict: &source_sync_conflict::Model,
    maps: &ImportIdMaps,
) -> Result<i64, ProjectFamilyImportError> {
    remap_source_sync_target_ref(
        &conflict.conflict_scope,
        &conflict.target_kind,
        conflict.target_id,
        maps,
    )
}

pub(in crate::project_family_import) fn remap_source_sync_target_ref(
    conflict_scope: &str,
    target_kind: &str,
    old_id: i64,
    maps: &ImportIdMaps,
) -> Result<i64, ProjectFamilyImportError> {
    match (conflict_scope, target_kind) {
        ("generation_spec", "segment") | ("text_anchor", "segment") => map_required(
            &maps.generation_spec_ids,
            old_id,
            "同步冲突 generation_spec target_id",
        ),
        ("text_anchor", "chapter") => {
            map_required(&maps.chapter_ids, old_id, "同步冲突 chapter target_id")
        }
        ("text_anchor", "galgame_scene") => map_required(
            &maps.galgame_scene_ids,
            old_id,
            "同步冲突 galgame_scene target_id",
        ),
        ("text_anchor", "galgame_line") => map_required(
            &maps.galgame_line_ids,
            old_id,
            "同步冲突 galgame_line target_id",
        ),
        ("profile", "character") => map_required(
            &maps.character_profile_ids,
            old_id,
            "同步冲突 character target_id",
        ),
        ("profile", "object") => map_required(
            &maps.object_profile_ids,
            old_id,
            "同步冲突 object target_id",
        ),
        ("profile", "scene") => {
            map_required(&maps.scene_profile_ids, old_id, "同步冲突 scene target_id")
        }
        _ => Err(ProjectFamilyImportError::invalid_package(format!(
            "同步冲突 target 引用类型无效: conflict_scope={conflict_scope}, target_kind={target_kind}"
        ))),
    }
}
