use super::{
    contracts::ExportFileData,
    scopes::{
        export_characters_json_task_scope, export_markdown_task_scope,
        export_outline_json_task_scope, export_txt_task_scope,
    },
};
use crate::AppState;

pub async fn export_txt_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ExportFileData, String> {
    export_txt_task_scope(state, user_id, project_id).await?;
    state
        .export()
        .run_export_txt_validated(project_id)
        .await
        .map_err(|error| error.to_string())
}

pub async fn export_markdown_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    title: &str,
    author: &str,
) -> Result<ExportFileData, String> {
    export_markdown_task_scope(state, user_id, project_id).await?;
    state
        .export()
        .run_export_markdown_validated(project_id, title, author)
        .await
        .map_err(|error| error.to_string())
}

pub async fn export_outline_json_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ExportFileData, String> {
    export_outline_json_task_scope(state, user_id, project_id).await?;
    state
        .export()
        .run_export_outline_json_validated(project_id)
        .await
        .map_err(|error| error.to_string())
}

pub async fn export_characters_json_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ExportFileData, String> {
    export_characters_json_task_scope(state, user_id, project_id).await?;
    state
        .export()
        .run_export_characters_json_validated(project_id)
        .await
        .map_err(|error| error.to_string())
}
