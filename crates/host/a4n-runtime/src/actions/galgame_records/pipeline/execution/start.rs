use crate::AppState;
use a4n_api_contract::galgame::{
    GalgameProgressReporter, GalgameScriptDetail, StartGalgamePipelineInput,
};

pub async fn start_galgame_pipeline(
    state: &AppState,
    user_id: i64,
    input: StartGalgamePipelineInput,
) -> Result<Option<GalgameScriptDetail>, String> {
    start_galgame_pipeline_with_progress(state, user_id, input, None).await
}

pub async fn start_galgame_pipeline_with_progress(
    state: &AppState,
    user_id: i64,
    input: StartGalgamePipelineInput,
    progress_reporter: Option<GalgameProgressReporter>,
) -> Result<Option<GalgameScriptDetail>, String> {
    let prompt_renderer = state;
    state
        .galgame_pipeline_execution_application()
        .start(user_id, input, progress_reporter, prompt_renderer)
        .await
        .map_err(|error| error.to_string())
}
