use super::{
    contracts::{BatchExportInput, ExportFileData, ExportProgress},
    scopes::export_batch_task_scope,
};
use crate::AppState;

pub async fn export_batch_data(
    state: &AppState,
    user_id: i64,
    input: BatchExportInput,
    progress: &ExportProgress<'_>,
) -> Result<Vec<ExportFileData>, String> {
    export_batch_task_scope(state, user_id, &input).await?;
    state
        .export()
        .run_batch_export_validated(input, progress)
        .await
        .map_err(|error| error.to_string())
}
