use super::{
    contracts::{ExportFileData, ExportProgress},
    scopes::export_pdf_task_scope,
};
use crate::AppState;

pub async fn export_pdf_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    progress: &ExportProgress<'_>,
) -> Result<ExportFileData, String> {
    export_pdf_task_scope(state, user_id, project_id).await?;
    state
        .export()
        .run_export_pdf_validated(project_id, progress)
        .await
        .map_err(|error| error.to_string())
}
