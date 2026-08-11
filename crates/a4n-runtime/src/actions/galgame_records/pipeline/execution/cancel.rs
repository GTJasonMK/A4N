use super::super::contracts::GalgamePipelineStatusSummary;
use super::super::status::get_galgame_pipeline_status;
use crate::{AppState, actions::galgame_records::GalgameProjectInput};

pub async fn cancel_galgame_pipeline(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgamePipelineStatusSummary, String> {
    state
        .orchestrator()
        .cancel_galgame_pipeline_for_user_validated(user_id, input.project_id)
        .await
        .map_err(|error| error.to_string())?;
    get_galgame_pipeline_status(state, user_id, input).await
}
