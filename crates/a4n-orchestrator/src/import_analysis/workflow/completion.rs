use serde_json::Value;

use super::super::{
    ImportProjectAnalysisSummary,
    progress::{finish_import_progress, save_import_progress},
};
use crate::OrchestratorContext;

pub async fn complete_import_project_analysis(
    context: &OrchestratorContext,
    project_id: i64,
    progress: &mut Value,
    analyzed_chapters: usize,
) -> Result<ImportProjectAnalysisSummary, String> {
    finish_import_progress(progress)?;
    save_import_progress(context, project_id, progress).await?;
    context
        .sync_project_status_from_artifacts(project_id)
        .await
        .map_err(|err| err.to_string())?;

    Ok(ImportProjectAnalysisSummary {
        progress: progress.clone(),
        analyzed_chapters,
    })
}
