use a4n_core::models::source_sync::source_sync_run;
use sea_orm::ConnectionTrait;

use super::cleanup::cleanup_galgame_scene_regeneration_records;
use super::persistence::persist_galgame_scene_regeneration_records;
use super::validation::validate_galgame_scene_regeneration_source_record;
use crate::source_sync::galgame::{
    SourceSyncGalgameSceneRegenerationApplyRecord, SourceSyncGalgameSceneRegenerationSourceRecord,
    SourceSyncGalgameSceneWriteRecordInput,
};
use crate::source_sync::{SourceSyncDbError, validate_positive_id};

pub async fn apply_galgame_scene_regeneration_record<C>(
    db: &C,
    run: &source_sync_run::Model,
    conflict_id: i64,
    source: SourceSyncGalgameSceneRegenerationSourceRecord,
    generated_scene: SourceSyncGalgameSceneWriteRecordInput,
) -> Result<SourceSyncGalgameSceneRegenerationApplyRecord, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(run.id, "source_sync_run_id")?;
    validate_positive_id(run.derived_project_id, "derived_project_id")?;
    validate_positive_id(run.to_snapshot_id, "to_snapshot_id")?;
    validate_positive_id(conflict_id, "source_sync_conflict_id")?;

    let scene = source.scene;
    let script = source.script;
    let old_snapshot_chapter = source.old_snapshot_chapter;
    let new_snapshot_chapter = source.new_snapshot_chapter;
    validate_galgame_scene_regeneration_source_record(
        run,
        &scene,
        &script,
        &old_snapshot_chapter,
        &new_snapshot_chapter,
    )?;

    let scene_id = scene.id;
    let cleanup =
        cleanup_galgame_scene_regeneration_records(db, run.derived_project_id, scene_id).await?;
    let persisted = persist_galgame_scene_regeneration_records(
        db,
        run,
        conflict_id,
        scene,
        script,
        new_snapshot_chapter.id,
        generated_scene,
        &cleanup.old_line_ids,
    )
    .await?;

    Ok(SourceSyncGalgameSceneRegenerationApplyRecord {
        galgame_scene_id: scene_id,
        old_source_snapshot_chapter_id: old_snapshot_chapter.id,
        new_source_snapshot_chapter_id: new_snapshot_chapter.id,
        old_line_count: cleanup.old_line_ids.len(),
        new_line_count: persisted.new_line_count,
        deleted_generation_spec_count: cleanup.deleted_generation_spec_count,
        deleted_portrait_generation_spec_count: cleanup.deleted_portrait_generation_spec_count,
        resolved_related_conflict_count: persisted.resolved_related_conflict_count,
    })
}
