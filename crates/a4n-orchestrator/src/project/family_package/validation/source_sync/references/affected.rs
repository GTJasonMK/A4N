use super::{
    super::contracts::SourceSyncReferenceSets,
    fields::{
        validate_optional_source_sync_json_id_ref, validate_optional_source_sync_profile_ref,
    },
    relationships::{
        validate_source_sync_anchor_snapshot_chapter_pair,
        validate_source_sync_galgame_line_scene_pair,
        validate_source_sync_snapshot_chapter_source_pair,
    },
};
use serde_json::{Map, Value};

pub(super) fn validate_source_sync_affected_refs(
    affected_refs: &Map<String, Value>,
    conflict_id: i64,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    validate_optional_source_sync_json_id_ref(
        affected_refs,
        "generation_spec_id",
        &refs.generation_spec_ids,
        "生成意图",
        "affected_refs_json",
        conflict_id,
    )?;
    let source_chapter_id = validate_optional_source_sync_json_id_ref(
        affected_refs,
        "source_chapter_id",
        &refs.chapter_ids,
        "源章节",
        "affected_refs_json",
        conflict_id,
    )?;
    let source_snapshot_chapter_id = validate_optional_source_sync_json_id_ref(
        affected_refs,
        "source_snapshot_chapter_id",
        &refs.source_snapshot_chapter_ids,
        "源快照章节",
        "affected_refs_json",
        conflict_id,
    )?;
    let old_source_snapshot_chapter_id = validate_optional_source_sync_json_id_ref(
        affected_refs,
        "old_source_snapshot_chapter_id",
        &refs.source_snapshot_chapter_ids,
        "源快照章节",
        "affected_refs_json",
        conflict_id,
    )?;
    let old_text_anchor_id = validate_optional_source_sync_json_id_ref(
        affected_refs,
        "old_text_anchor_id",
        &refs.text_anchor_ids,
        "文本锚点",
        "affected_refs_json",
        conflict_id,
    )?;
    let new_text_anchor_id = validate_optional_source_sync_json_id_ref(
        affected_refs,
        "new_text_anchor_id",
        &refs.text_anchor_ids,
        "文本锚点",
        "affected_refs_json",
        conflict_id,
    )?;
    let galgame_scene_id = validate_optional_source_sync_json_id_ref(
        affected_refs,
        "galgame_scene_id",
        &refs.galgame_scene_ids,
        "Galgame 场",
        "affected_refs_json",
        conflict_id,
    )?;
    let galgame_line_id = validate_optional_source_sync_json_id_ref(
        affected_refs,
        "galgame_line_id",
        &refs.galgame_line_ids,
        "Galgame 行",
        "affected_refs_json",
        conflict_id,
    )?;
    validate_source_sync_snapshot_chapter_source_pair(
        source_snapshot_chapter_id,
        source_chapter_id,
        "affected_refs_json",
        conflict_id,
        refs,
    )?;
    validate_source_sync_anchor_snapshot_chapter_pair(
        old_text_anchor_id,
        old_source_snapshot_chapter_id,
        "old_text_anchor_id",
        "old_source_snapshot_chapter_id",
        "affected_refs_json",
        conflict_id,
        refs,
    )?;
    validate_source_sync_anchor_snapshot_chapter_pair(
        new_text_anchor_id,
        source_snapshot_chapter_id,
        "new_text_anchor_id",
        "source_snapshot_chapter_id",
        "affected_refs_json",
        conflict_id,
        refs,
    )?;
    validate_source_sync_galgame_line_scene_pair(
        galgame_line_id,
        galgame_scene_id,
        "affected_refs_json",
        conflict_id,
        refs,
    )?;
    validate_optional_source_sync_profile_ref(
        affected_refs,
        "affected_refs_json",
        conflict_id,
        refs,
    )
}
