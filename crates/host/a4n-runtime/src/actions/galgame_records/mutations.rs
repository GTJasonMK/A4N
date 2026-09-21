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
        .galgame_record_mutation_application()
        .update_line(user_id, project_id, input)
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
        .galgame_record_mutation_application()
        .insert_line(user_id, project_id, input)
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
        .galgame_record_mutation_application()
        .update_scene(user_id, project_id, input)
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
        .galgame_record_mutation_application()
        .delete_line(user_id, project_id, input)
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
        .galgame_record_mutation_application()
        .reorder_lines(user_id, project_id, input)
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
        .galgame_record_mutation_application()
        .update_choices(user_id, project_id, input)
        .await
        .map_err(|err| err.to_string())
}
