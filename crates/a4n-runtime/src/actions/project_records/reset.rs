mod summaries;

use self::summaries::{to_freeform_generated_data_reset_summary, to_generated_data_reset_summary};
use super::contracts::ProjectGeneratedDataResetSummary;
use crate::AppState;

pub(super) async fn reset_generated_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ProjectGeneratedDataResetSummary, String> {
    let reset = state
        .destructive_lifecycle()
        .reset_project(state, user_id, project_id)
        .await?;
    match reset {
        a4n_orchestrator::project::ProjectGeneratedDataResetApplicationResult::Standard(
            outcome,
        ) => {
            to_generated_data_reset_summary(
                state,
                user_id,
                &outcome.result,
                outcome.media_paths.len(),
            )
            .await
        }
        a4n_orchestrator::project::ProjectGeneratedDataResetApplicationResult::Freeform {
            project: freeform_project,
            result,
        } => {
            to_freeform_generated_data_reset_summary(
                state,
                user_id,
                &freeform_project,
                &result,
                result.media_paths.len(),
            )
            .await
        }
    }
}
