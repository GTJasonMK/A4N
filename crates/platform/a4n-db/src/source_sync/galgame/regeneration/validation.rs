use std::collections::HashSet;

use a4n_db_models::models::{
    galgame::{galgame_scene, galgame_script},
    source_sync::source_sync_run,
};
use a4n_domain::project::SourceSnapshotChapterSummary;

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
    validate_positive_id(generated_scene.adaptation_plan_id, "adaptation_plan_id")?;
    if generated_scene.adaptation_plan_revision <= 0
        || generated_scene.adaptation_unit_id.trim().is_empty()
    {
        return Err(SourceSyncDbError::InvalidArgument(
            "场景改编来源版本或单元身份无效".into(),
        ));
    }
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
        .collect::<HashSet<_>>();
    for line in &generated_scene.lines {
        let presentation: a4n_domain::galgame::GalgameLinePresentationDocumentV3 =
            serde_json::from_value(line.presentation_json.clone())
                .map_err(|error| SourceSyncDbError::InvalidArgument(error.to_string()))?;
        presentation
            .validate_source_identity(line.text_anchor_id)
            .map_err(SourceSyncDbError::InvalidArgument)?;
        let Some(text_anchor_id) = line.text_anchor_id else {
            if matches!(
                presentation.origin,
                a4n_domain::media_authoring::AdaptationOrigin::Added
                    | a4n_domain::media_authoring::AdaptationOrigin::Historical
            ) {
                continue;
            }
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
    old_snapshot_chapter: &SourceSnapshotChapterSummary,
    new_snapshot_chapter: &SourceSnapshotChapterSummary,
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
    if old_snapshot_chapter.root_project_id != run.root_project_id
        || new_snapshot_chapter.root_project_id != run.root_project_id
        || old_snapshot_chapter.source_chapter_id != new_snapshot_chapter.source_chapter_id
    {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "Galgame 场 {} 的新旧快照必须来自同一根项目章节",
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
