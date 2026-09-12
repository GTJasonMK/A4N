use crate::{AppState, actions::galgame_records::GalgameProjectInput};
use a4n_api_contract::galgame::GalgamePipelineStatusSummary;

pub async fn cancel_galgame_pipeline(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgamePipelineStatusSummary, String> {
    state
        .galgame_pipeline_cancellation_application()
        .cancel(user_id, input.project_id)
        .await
        .map_err(|error| error.to_string())
}
