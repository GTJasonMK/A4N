use serde_json::Value;

use super::{
    ImportProjectAnalysisSummary,
    progress::{cancel_import_progress, load_import_progress, load_import_progress_optional},
};
use crate::OrchestratorContext;

pub async fn project_import_analysis_progress(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<Option<Value>, String> {
    load_import_progress_optional(context, project_id).await
}

pub async fn cancel_import_analysis(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<(), String> {
    cancel_import_progress(context, project_id).await?;
    Ok(())
}

pub async fn cancelled_import_analysis_summary(
    context: &OrchestratorContext,
    project_id: i64,
    analyzed_chapters: usize,
) -> Result<ImportProjectAnalysisSummary, String> {
    Ok(ImportProjectAnalysisSummary {
        progress: load_import_progress(context, project_id).await?,
        analyzed_chapters,
    })
}
