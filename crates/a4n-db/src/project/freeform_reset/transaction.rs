use super::contracts::FreeformGeneratedResetDbError;
use super::targets::freeform_generated_reset_targets_in;
use crate::project::cleanup::{
    delete_project_generated_artifact_links, delete_project_generated_artifact_rows,
    delete_project_generation_runs, delete_project_task_queue_records,
    delete_project_workflow_runs,
};
use crate::project::{project_by_id_for_update, validate_positive_i64};
use a4n_domain::project::ProjectSourceMode;
use a4n_domain::project_reset::FreeformVideoGeneratedDataResetResult;
use sea_orm::{ConnectionTrait, TransactionTrait};

pub async fn reset_freeform_video_generated_data_records<C>(
    db: &C,
    project_id: i64,
) -> Result<FreeformVideoGeneratedDataResetResult, FreeformGeneratedResetDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    validate_positive_i64(project_id, "project_id")
        .map_err(FreeformGeneratedResetDbError::InvalidArgument)?;
    let txn = db.begin().await?;
    let project = project_by_id_for_update(&txn, project_id)
        .await
        .map_err(FreeformGeneratedResetDbError::from)?
        .ok_or(FreeformGeneratedResetDbError::ProjectNotFound { project_id })?;
    crate::generation::require_project_lifecycle_gate(
        &txn,
        project_id,
        crate::generation::PROJECT_GATE_RESETTING,
    )
    .await
    .map_err(|error| FreeformGeneratedResetDbError::Database {
        source: sea_orm::DbErr::Custom(error.to_string()),
    })?;
    if project.source_mode().map_err(|error| {
        FreeformGeneratedResetDbError::InvalidArgument(format!("项目 {project_id}: {error}"))
    })? != ProjectSourceMode::Freeform
    {
        return Err(FreeformGeneratedResetDbError::UnsupportedProject);
    }

    let targets = freeform_generated_reset_targets_in(&txn, project_id, true).await?;
    delete_project_generation_runs(&txn, project_id, &targets.image_ids, &targets.video_ids)
        .await?;
    let task_cleanup = delete_project_task_queue_records(&txn, project_id).await?;
    let artifact_link_cleanup = delete_project_generated_artifact_links(
        &txn,
        project_id,
        &targets.media_asset_ids,
        &targets.generation_spec_ids,
        &targets.prompt_artifact_ids,
    )
    .await?;
    delete_project_workflow_runs(&txn, project_id).await?;
    crate::canvas::mark_freeform_canvas_generated_refs_missing(&txn, project_id, &targets).await?;
    let artifact_row_cleanup = delete_project_generated_artifact_rows(
        &txn,
        project_id,
        &targets.image_ids,
        &targets.video_ids,
        &targets.media_asset_ids,
        &targets.generation_spec_ids,
    )
    .await?;
    let media_paths = crate::media::schedule_unreferenced_media_cleanup(
        &txn,
        project_id,
        &targets.media_paths,
        format!("project-generated-data-reset:{project_id}"),
    )
    .await?;
    crate::generation::complete_project_reset_gate(&txn, project_id)
        .await
        .map_err(|error| FreeformGeneratedResetDbError::Database {
            source: sea_orm::DbErr::Custom(error.to_string()),
        })?;
    txn.commit().await?;

    Ok(FreeformVideoGeneratedDataResetResult {
        deleted_tasks: task_cleanup.deleted_tasks,
        deleted_task_dependencies: task_cleanup.deleted_task_dependencies,
        deleted_generation_specs: artifact_row_cleanup.deleted_generation_specs,
        deleted_prompt_artifacts: artifact_link_cleanup.deleted_prompt_artifacts,
        deleted_media_assets: artifact_row_cleanup.deleted_media_assets,
        deleted_media_provenance: artifact_link_cleanup.deleted_media_provenance,
        deleted_media_bindings: artifact_link_cleanup.deleted_media_bindings,
        deleted_generated_images: artifact_row_cleanup.deleted_generated_images,
        deleted_generated_videos: artifact_row_cleanup.deleted_generated_videos,
        media_paths,
    })
}
