use std::collections::BTreeSet;

use a4n_core::models::{
    galgame::{galgame_scene, galgame_script},
    project::source_snapshot_chapter,
    source_sync::source_sync_run,
};

use crate::source_sync::galgame::{
    SourceSyncGalgameSceneRegenerationSourceRecord, SourceSyncGalgameSceneWriteRecordInput,
};
use crate::source_sync::{SourceSyncDbError, validate_positive_id};

pub(in crate::source_sync) fn validate_prepared_galgame_scene_regeneration_record(
    source: &SourceSyncGalgameSceneRegenerationSourceRecord,
    expected_scene_index: i32,
    expected_source_snapshot_chapter_id: i64,
    generated_scene: &SourceSyncGalgameSceneWriteRecordInput,
) -> Result<(), SourceSyncDbError> {
    if source.scene.scene_index != expected_scene_index {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "Galgame source sync 重建计划 scene_index 不匹配: expected={}, actual={}",
            source.scene.scene_index, expected_scene_index
        )));
    }
    if source.new_snapshot_chapter.id != expected_source_snapshot_chapter_id {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "Galgame source sync 重建计划 source_snapshot_chapter_id 不匹配: expected={}, actual={}",
            source.new_snapshot_chapter.id, expected_source_snapshot_chapter_id
        )));
    }
    let valid_anchor_ids = source
        .anchors
        .iter()
        .map(|anchor| anchor.id)
        .collect::<BTreeSet<_>>();
    for line in &generated_scene.lines {
        let Some(text_anchor_id) = line.text_anchor_id else {
            return Err(SourceSyncDbError::InvalidArgument(format!(
                "Galgame source sync 重建计划第 {} 行缺少 text_anchor_id",
                line.line_index
            )));
        };
        if !valid_anchor_ids.contains(&text_anchor_id) {
            return Err(SourceSyncDbError::InvalidArgument(format!(
                "Galgame source sync 重建计划第 {} 行 text_anchor_id 不属于目标源快照章节: {}",
                line.line_index, text_anchor_id
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_galgame_scene_regeneration_source_record(
    run: &source_sync_run::Model,
    scene: &galgame_scene::Model,
    script: &galgame_script::Model,
    old_snapshot_chapter: &source_snapshot_chapter::Model,
    new_snapshot_chapter: &source_snapshot_chapter::Model,
) -> Result<(), SourceSyncDbError> {
    validate_positive_id(scene.id, "galgame_scene_id")?;
    validate_positive_id(script.id, "galgame_script_id")?;
    validate_positive_id(old_snapshot_chapter.id, "old_source_snapshot_chapter_id")?;
    validate_positive_id(new_snapshot_chapter.id, "new_source_snapshot_chapter_id")?;
    if scene.script_id != script.id {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "Galgame 场 {} 不属于脚本 {}",
            scene.id, script.id
        )));
    }
    if script.project_id != run.derived_project_id || script.root_project_id != run.root_project_id
    {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "Galgame 场 {} 不属于本次源同步运行",
            scene.id
        )));
    }
    if scene.source_snapshot_chapter_id != old_snapshot_chapter.id {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "Galgame 场 {} 的源快照章节与读取记录不一致",
            scene.id
        )));
    }
    if old_snapshot_chapter.snapshot_id != run.from_snapshot_id {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "Galgame 场 {} 的源快照章节不属于本次同步起点",
            scene.id
        )));
    }
    if new_snapshot_chapter.snapshot_id != run.to_snapshot_id {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "Galgame 场 {} 的目标源快照章节不属于本次同步终点",
            scene.id
        )));
    }
    Ok(())
}
