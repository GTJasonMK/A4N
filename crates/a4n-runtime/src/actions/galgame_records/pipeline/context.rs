use crate::AppState;

pub(super) async fn load_galgame_pipeline_prompt_context(
    state: &AppState,
    project_id: i64,
) -> Result<a4n_orchestrator::galgame::GalgamePipelinePromptContext, String> {
    state
        .orchestrator
        .galgame_pipeline_prompt_context(project_id)
        .await
        .map_err(|err| err.to_string())
}
