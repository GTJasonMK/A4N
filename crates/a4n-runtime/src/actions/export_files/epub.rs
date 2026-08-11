use super::{
    contracts::{ExportFileData, ExportProgress},
    scopes::export_epub_task_scope,
};
use crate::AppState;

pub async fn export_epub_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    title: String,
    author: String,
    progress: &ExportProgress<'_>,
) -> Result<ExportFileData, String> {
    export_epub_task_scope(state, user_id, project_id).await?;
    state
        .orchestrator()
        .run_export_epub_validated(project_id, &title, &author, progress)
        .await
        .map_err(|error| error.to_string())
}
