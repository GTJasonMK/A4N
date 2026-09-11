mod summaries;

use self::summaries::{to_freeform_generated_data_reset_summary, to_generated_data_reset_summary};
use crate::AppState;
use a4n_api_contract::project::{
    ProjectGeneratedDataResetResponse, ProjectGeneratedDataResetStatus,
};

pub(super) async fn reset_generated_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ProjectGeneratedDataResetResponse, String> {
    let reset = state
        .destructive_lifecycle()
        .reset_project(state, user_id, project_id)
        .await?;
    let result = match reset {
        a4n_orchestrator::project::ProjectGeneratedDataResetApplicationResult::Standard(
            outcome,
        ) => {
            to_generated_data_reset_summary(
                state,
                user_id,
                project_id,
                &outcome.result,
                outcome.media_paths.len(),
            )
            .await
        }
        a4n_orchestrator::project::ProjectGeneratedDataResetApplicationResult::Freeform {
            result,
        } => {
            to_freeform_generated_data_reset_summary(
                state,
                user_id,
                project_id,
                &result,
                result.media_paths.len(),
            )
            .await
        }
    }?;
    Ok(ProjectGeneratedDataResetResponse {
        status: ProjectGeneratedDataResetStatus::Completed,
        database_committed: true,
        result: Some(result),
        warnings: Vec::new(),
    })
}
