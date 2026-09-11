use super::super::{GalgamePipelineStatusSummary, GalgameProjectInput};
use crate::AppState;

pub async fn get_galgame_pipeline_status(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgamePipelineStatusSummary, String> {
    state
        .galgame_pipeline_application()
        .status(user_id, input.project_id)
        .await
        .map_err(|error| error.to_string())
}
