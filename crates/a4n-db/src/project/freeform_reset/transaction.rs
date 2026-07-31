use super::contracts::{FreeformGeneratedResetDbError, FreeformVideoGeneratedDataResetResult};
use super::targets::freeform_generated_reset_targets_in;
use crate::project::cleanup::{
    delete_project_generated_artifact_links, delete_project_generated_artifact_rows,
    delete_project_generation_runs, delete_project_task_queue_records,
    delete_project_workflow_runs,
};
use crate::project::{
    PROJECT_KIND_VIDEO, SOURCE_MODE_FREEFORM, project_by_id_for_update, validate_positive_i64,
};
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
    if project.project_kind != PROJECT_KIND_VIDEO || project.source_mode != SOURCE_MODE_FREEFORM {
        return Err(FreeformGeneratedResetDbError::UnsupportedProject {
            project_kind: project.project_kind,
            source_mode: project.source_mode,
        });
    }

    let targets = freeform_generated_reset_targets_in(&txn, project_id, true).await?;
    let _generation_cleanup =
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
    let _workflow_cleanup = delete_project_workflow_runs(&txn, project_id).await?;
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
        task_cleanup,
        artifact_link_cleanup,
        artifact_row_cleanup,
        media_paths,
    })
}
