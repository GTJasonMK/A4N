mod cleanup;
mod summaries;

use self::cleanup::clear_rag_and_remove_media_files;
use self::summaries::{to_freeform_generated_data_reset_summary, to_generated_data_reset_summary};
use super::contracts::ProjectGeneratedDataResetSummary;
use super::summaries::project_record_by_id;
use crate::AppState;
use a4n_core::models::project::novel_project;
use a4n_orchestrator::project::SOURCE_MODE_FREEFORM;

const PROJECT_KIND_VIDEO: &str = "video";

pub(super) async fn reset_generated_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ProjectGeneratedDataResetSummary, String> {
    let project = project_record_by_id(state, project_id).await?;
    if project.project_kind == PROJECT_KIND_VIDEO && project.source_mode == SOURCE_MODE_FREEFORM {
        let (reset, media_paths) =
            reset_freeform_video_generated_data(state, user_id, &project).await?;
        clear_rag_and_remove_media_files(state, project_id, &media_paths).await?;
        return Ok(reset);
    }
    let reset = state
        .orchestrator
        .reset_project_generated_data(project_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    clear_rag_and_remove_media_files(state, project_id, &reset.media_paths).await?;
    to_generated_data_reset_summary(state, user_id, &reset.result, reset.media_paths.len()).await
}

async fn reset_freeform_video_generated_data(
    state: &AppState,
    user_id: i64,
    project: &novel_project::Model,
) -> Result<(ProjectGeneratedDataResetSummary, Vec<String>), String> {
    let reset_result = state
        .orchestrator
        .reset_freeform_video_generated_data_records(project.id)
        .await
        .map_err(|error| error.to_string())?;
    let summary = to_freeform_generated_data_reset_summary(
        state,
        user_id,
        project,
        &reset_result,
        reset_result.media_paths.len(),
    )
    .await?;
    Ok((summary, reset_result.media_paths))
}
