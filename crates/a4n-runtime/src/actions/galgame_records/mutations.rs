use crate::AppState;
use a4n_core::models::galgame::{galgame_choice, galgame_line};
use a4n_orchestrator::galgame::{
    InsertGalgameLineInput, ReorderGalgameLinesInput, UpdateGalgameChoicesInput,
    UpdateGalgameLineInput, UpdateGalgameSceneInput,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeleteGalgameLineInput {
    pub line_id: i64,
}

pub async fn update_galgame_line(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: UpdateGalgameLineInput,
) -> Result<galgame_line::Model, String> {
    state
        .galgame()
        .update_galgame_line_for_user_validated(user_id, project_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn insert_galgame_line(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: InsertGalgameLineInput,
) -> Result<galgame_line::Model, String> {
    state
        .galgame()
        .insert_galgame_line_for_user_validated(user_id, project_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn update_galgame_scene(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: UpdateGalgameSceneInput,
) -> Result<a4n_core::models::galgame::galgame_scene::Model, String> {
    state
        .galgame()
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
        .galgame()
        .delete_galgame_line_for_user_validated(user_id, project_id, input.line_id)
        .await
        .map_err(|err| err.to_string())
}

pub async fn reorder_galgame_lines(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: ReorderGalgameLinesInput,
) -> Result<Vec<galgame_line::Model>, String> {
    state
        .galgame()
        .reorder_galgame_lines_for_user_validated(user_id, project_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn update_galgame_choices(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: UpdateGalgameChoicesInput,
) -> Result<Vec<galgame_choice::Model>, String> {
    state
        .galgame()
        .update_galgame_choices_for_user_validated(user_id, project_id, input)
        .await
        .map_err(|err| err.to_string())
}
