use a4n_error::AppError;

use a4n_api_contract::export::{BatchExportInput, ExportFileData};
pub use a4n_orchestrator::export::ExportProgress;

use crate::AppState;

pub async fn export_txt_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ExportFileData, AppError> {
    state
        .export_application()
        .export_txt(user_id, project_id)
        .await
}

pub async fn export_markdown_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    title: &str,
    author: &str,
) -> Result<ExportFileData, AppError> {
    state
        .export_application()
        .export_markdown(user_id, project_id, title, author)
        .await
}

pub async fn export_outline_json_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ExportFileData, AppError> {
    state
        .export_application()
        .export_outline_json(user_id, project_id)
        .await
}

pub async fn export_characters_json_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ExportFileData, AppError> {
    state
        .export_application()
        .export_characters_json(user_id, project_id)
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
    state
        .export_application()
        .export_epub(user_id, project_id, title, author, progress)
        .await
}

pub async fn export_pdf_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    progress: &ExportProgress<'_>,
) -> Result<ExportFileData, AppError> {
    state
        .export_application()
        .export_pdf(user_id, project_id, progress)
        .await
}

pub async fn export_batch_data(
    state: &AppState,
    user_id: i64,
    input: BatchExportInput,
    progress: &ExportProgress<'_>,
) -> Result<Vec<ExportFileData>, AppError> {
    state
        .export_application()
        .export_batch(user_id, input, progress)
        .await
}
