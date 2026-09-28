use a4n_db_models::models::source_sync::source_sync_run;
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
    let document = crate::media_authoring::authoring_document(
        db,
        run.derived_project_id,
        generated_scene.adaptation_plan_id,
    )
    .await
    .map_err(|error| match error {
        crate::media_authoring::AuthoringDocumentError::Database(error) => {
            SourceSyncDbError::Database(error)
        }
        error => SourceSyncDbError::InvalidArgument(error.to_string()),
    })?;
    if document.kind != a4n_domain::media_authoring::AuthoringDocumentKind::AdaptationPlan
        || document.status == a4n_domain::media_authoring::AuthoringDocumentStatus::Draft
        || document.source_snapshot_id != new_snapshot_chapter.snapshot_id
        || document.source_chapter_id != Some(new_snapshot_chapter.source_chapter_id)
        || document.revision != generated_scene.adaptation_plan_revision
        || scene.adaptation_unit_id.as_deref() != Some(generated_scene.adaptation_unit_id.as_str())
    {
        return Err(SourceSyncDbError::InvalidArgument(
            "源同步场景的改编来源不匹配".into(),
        ));
    }
    let plan: a4n_domain::media_authoring::AdaptationPlanV1 =
        serde_json::from_value(document.document)
            .map_err(|error| SourceSyncDbError::InvalidArgument(error.to_string()))?;
    if plan.target != a4n_domain::media_authoring::AdaptationTarget::Galgame
        || !plan
            .units
            .iter()
            .any(|unit| unit.id == generated_scene.adaptation_unit_id)
    {
        return Err(SourceSyncDbError::InvalidArgument(
            "源同步目标改编单元不存在".into(),
        ));
    }
    for line in &generated_scene.lines {
        let presentation: a4n_domain::galgame::GalgameLinePresentationDocumentV3 =
            serde_json::from_value(line.presentation_json.clone())
                .map_err(|error| SourceSyncDbError::InvalidArgument(error.to_string()))?;
        presentation
            .validate_adaptation(
                &plan,
                new_snapshot_chapter.source_chapter_id,
                &generated_scene.adaptation_unit_id,
                line.text_anchor_id,
            )
            .map_err(SourceSyncDbError::InvalidArgument)?;
    }

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
