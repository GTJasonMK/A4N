use super::{
    DeleteGalgameLineInput, GalgameChoice, GalgameLine, GalgameScene, InsertGalgameLineInput,
    ReorderGalgameLinesInput, UpdateGalgameChoicesInput, UpdateGalgameLineInput,
    UpdateGalgameSceneInput,
};
use crate::AppState;

pub async fn update_galgame_line(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: UpdateGalgameLineInput,
) -> Result<GalgameLine, String> {
    state
        .orchestrator()
        .update_galgame_line_for_user_validated(user_id, project_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn insert_galgame_line(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: InsertGalgameLineInput,
) -> Result<GalgameLine, String> {
    state
        .orchestrator()
        .insert_galgame_line_for_user_validated(user_id, project_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn update_galgame_scene(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: UpdateGalgameSceneInput,
) -> Result<GalgameScene, String> {
    state
        .orchestrator()
        .update_galgame_scene_for_user_validated(user_id, project_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn delete_galgame_line(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: DeleteGalgameLineInput,
) -> Result<(), String> {
    state
        .orchestrator()
        .delete_galgame_line_for_user_validated(user_id, project_id, input.line_id)
        .await
        .map_err(|err| err.to_string())
}

pub async fn reorder_galgame_lines(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: ReorderGalgameLinesInput,
) -> Result<Vec<GalgameLine>, String> {
    state
        .orchestrator()
        .reorder_galgame_lines_for_user_validated(user_id, project_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn update_galgame_choices(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: UpdateGalgameChoicesInput,
) -> Result<Vec<GalgameChoice>, String> {
    state
        .orchestrator()
        .update_galgame_choices_for_user_validated(user_id, project_id, input)
        .await
        .map_err(|err| err.to_string())
}
