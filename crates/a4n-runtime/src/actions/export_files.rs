use a4n_core::error::AppError;

use a4n_api_contract::export::{BatchExportInput, ExportFileData};
pub use a4n_orchestrator::export::ExportProgress;

use crate::{AppState, actions::access::verify_project_editor};

pub async fn export_txt_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ExportFileData, AppError> {
    verify_project_editor(state, user_id, project_id).await?;
    state
        .orchestrator()
        .run_export_txt_validated(project_id)
        .await
}

pub async fn export_markdown_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    title: &str,
    author: &str,
) -> Result<ExportFileData, AppError> {
    verify_project_editor(state, user_id, project_id).await?;
    state
        .orchestrator()
        .run_export_markdown_validated(project_id, title, author)
        .await
}

pub async fn export_outline_json_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ExportFileData, AppError> {
    verify_project_editor(state, user_id, project_id).await?;
    state
        .orchestrator()
        .run_export_outline_json_validated(project_id)
        .await
}

pub async fn export_characters_json_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ExportFileData, AppError> {
    verify_project_editor(state, user_id, project_id).await?;
    state
        .orchestrator()
        .run_export_characters_json_validated(project_id)
        .await
}

pub async fn export_epub_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    title: &str,
    author: &str,
    progress: &ExportProgress<'_>,
) -> Result<ExportFileData, AppError> {
    verify_project_editor(state, user_id, project_id).await?;
    state
        .orchestrator()
        .run_export_epub_validated(project_id, title, author, progress)
        .await
}

pub async fn export_pdf_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    progress: &ExportProgress<'_>,
) -> Result<ExportFileData, AppError> {
    verify_project_editor(state, user_id, project_id).await?;
    state
        .orchestrator()
        .run_export_pdf_validated(project_id, progress)
        .await
}

pub async fn export_batch_data(
    state: &AppState,
    user_id: i64,
    input: BatchExportInput,
    progress: &ExportProgress<'_>,
) -> Result<Vec<ExportFileData>, AppError> {
    verify_project_editor(state, user_id, input.project_id).await?;
    state
        .orchestrator()
        .run_batch_export_validated(input, progress)
        .await
}
