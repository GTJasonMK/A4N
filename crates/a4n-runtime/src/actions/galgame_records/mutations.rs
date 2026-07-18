use crate::{AppState, actions::access};
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
    access::verify_project_editor_string(state, user_id, project_id).await?;
    state
        .update_galgame_line(project_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn insert_galgame_line(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: InsertGalgameLineInput,
) -> Result<galgame_line::Model, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    state
        .insert_galgame_line(project_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn update_galgame_scene(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: UpdateGalgameSceneInput,
) -> Result<a4n_core::models::galgame::galgame_scene::Model, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    state
        .update_galgame_scene(project_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn delete_galgame_line(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: DeleteGalgameLineInput,
) -> Result<(), String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    state
        .orchestrator
        .delete_galgame_line(project_id, input.line_id)
        .await
        .map_err(|err| err.to_string())
}

pub async fn reorder_galgame_lines(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: ReorderGalgameLinesInput,
) -> Result<Vec<galgame_line::Model>, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    state
        .orchestrator
        .reorder_galgame_lines(project_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn update_galgame_choices(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: UpdateGalgameChoicesInput,
) -> Result<Vec<galgame_choice::Model>, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    state
        .update_galgame_choices(project_id, input)
        .await
        .map_err(|err| err.to_string())
}
