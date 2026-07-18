use a4n_core::models::generation::generation_spec;

use crate::project_family_import::{
    ImportIdMaps, ProjectFamilyImportError, map_required, remap_profile_id,
};

pub fn remap_generation_target_id(
    spec: &generation_spec::Model,
    maps: &ImportIdMaps,
    allow_deferred: bool,
) -> Result<Option<i64>, ProjectFamilyImportError> {
    let Some(old_id) = spec.target_entity_id else {
        return Ok(None);
    };
    let mapped = match spec.target_entity_kind.as_str() {
        "character" | "object" | "scene" => remap_profile_id(
            &spec.target_entity_kind,
            old_id,
            maps,
            "生成意图 target_entity_id",
        )?,
        "character_profile" => map_required(
            &maps.character_profile_ids,
            old_id,
            "生成意图 character_profile target_entity_id",
        )?,
        "chapter" => map_required(
            &maps.source_snapshot_chapter_ids,
            old_id,
            "生成意图 chapter target_entity_id",
        )?,
        "galgame_scene" => map_required(
            &maps.galgame_scene_ids,
            old_id,
            "生成意图 galgame_scene target_entity_id",
        )?,
        "galgame_line" => map_required(
            &maps.galgame_line_ids,
            old_id,
            "生成意图 galgame_line target_entity_id",
        )?,
        "segment" if allow_deferred => map_required(
            &maps.video_segment_ids,
            old_id,
            "生成意图 segment target_entity_id",
        )?,
        "assembly" if allow_deferred => map_required(
            &maps.assembly_timeline_ids,
            old_id,
            "生成意图 assembly target_entity_id",
        )?,
        "segment" | "assembly" => return Ok(None),
        other => {
            return Err(ProjectFamilyImportError::invalid_package(format!(
                "生成意图 target_entity_kind 无效: {other}"
            )));
        }
    };
    Ok(Some(mapped))
}

pub fn remap_generation_spec_source_version_hash(
    spec: &generation_spec::Model,
    maps: &ImportIdMaps,
) -> Result<String, ProjectFamilyImportError> {
    let source_snapshot_id = spec.source_snapshot_id.ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(format!(
            "生成意图 source_version_hash 缺少源快照: generation_spec_id={}",
            spec.id
        ))
    })?;
    maps.source_snapshot_version_hashes
        .get(&source_snapshot_id)
        .cloned()
        .ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!(
                "生成意图 source_version_hash 引用了导入包外或尚未导入的源快照: {}",
                source_snapshot_id
            ))
        })
}
