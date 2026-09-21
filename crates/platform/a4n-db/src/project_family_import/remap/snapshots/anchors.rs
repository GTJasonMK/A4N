use std::collections::HashMap;

use a4n_db_models::models::project::text_anchor;

use crate::project_family_import::{ImportIdMaps, ProjectFamilyImportError, map_required};

pub(in crate::project_family_import) fn remap_text_anchor_source_id(
    anchor: &text_anchor::Model,
    maps: &ImportIdMaps,
) -> Result<i64, ProjectFamilyImportError> {
    match anchor.source_kind.as_str() {
        "chapter_content" | "chapter_summary" => {
            if anchor.source_snapshot_id.is_some() {
                map_required(
                    &maps.source_snapshot_chapter_ids,
                    anchor.source_id,
                    "文本锚点 source_id",
                )
            } else {
                map_required(&maps.chapter_ids, anchor.source_id, "文本锚点 source_id")
            }
        }
        "outline" => remap_ambiguous_outline_id(anchor.source_id, maps),
        "blueprint" => remap_ambiguous_blueprint_id(anchor.source_id, maps),
        other => Err(ProjectFamilyImportError::invalid_package(format!(
            "文本锚点 source_kind 无效: {other}"
        ))),
    }
}

fn remap_ambiguous_outline_id(
    old_id: i64,
    maps: &ImportIdMaps,
) -> Result<i64, ProjectFamilyImportError> {
    remap_ambiguous_id(
        old_id,
        &maps.part_outline_ids,
        &maps.chapter_outline_ids,
        "文本锚点 outline source_id",
    )
}

fn remap_ambiguous_blueprint_id(
    old_id: i64,
    maps: &ImportIdMaps,
) -> Result<i64, ProjectFamilyImportError> {
    remap_ambiguous_id(
        old_id,
        &maps.blueprint_ids,
        &maps.blueprint_version_ids,
        "文本锚点 blueprint source_id",
    )
}

fn remap_ambiguous_id(
    old_id: i64,
    first: &HashMap<i64, i64>,
    second: &HashMap<i64, i64>,
    label: &str,
) -> Result<i64, ProjectFamilyImportError> {
    match (first.get(&old_id).copied(), second.get(&old_id).copied()) {
        (Some(id), None) | (None, Some(id)) => Ok(id),
        (Some(_), Some(_)) => Err(ProjectFamilyImportError::invalid_package(format!(
            "{label} 存在歧义: {old_id}"
        ))),
        (None, None) => Err(ProjectFamilyImportError::invalid_package(format!(
            "{label} 不存在: {old_id}"
        ))),
    }
}
