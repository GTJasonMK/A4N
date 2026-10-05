use super::{
    AdvanceGalgameStoryRuntimeInput, GalgameStoryRuntimeView, GetGalgameStoryRuntimeInput,
    StartGalgameStoryRuntimeInput,
};
use crate::AppState;

pub async fn get_galgame_story_runtime_state(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: GetGalgameStoryRuntimeInput,
) -> Result<Option<GalgameStoryRuntimeView>, String> {
    state
        .galgame_story_graph_mutation_application()
        .runtime_state(user_id, project_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn start_galgame_story_runtime(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: StartGalgameStoryRuntimeInput,
) -> Result<GalgameStoryRuntimeView, String> {
    state
        .galgame_story_graph_mutation_application()
        .start_runtime(user_id, project_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn advance_galgame_story_runtime(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: AdvanceGalgameStoryRuntimeInput,
) -> Result<GalgameStoryRuntimeView, String> {
    state
        .galgame_story_graph_mutation_application()
        .advance_runtime(user_id, project_id, input)
        .await
        .map_err(|err| err.to_string())
}
