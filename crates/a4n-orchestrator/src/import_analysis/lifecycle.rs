use serde_json::Value;

use super::{
    ImportProjectAnalysisSummary,
    progress::{cancel_import_progress, load_import_progress, load_import_progress_optional},
    summary::completed_import_part_outlines,
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
) -> Result<u64, String> {
    let rows_affected = context
        .delete_pending_import_reviews_by_project(project_id)
        .await
        .map_err(|err| err.to_string())?;

    cancel_import_progress(context, project_id).await?;
    Ok(rows_affected)
}

pub async fn cancelled_import_analysis_summary(
    context: &OrchestratorContext,
    project_id: i64,
    progress: &Value,
    analyzed_chapters: usize,
    outlined_chapters: usize,
    needs_part_outlines: bool,
    chapter_count: usize,
) -> Result<ImportProjectAnalysisSummary, String> {
    Ok(ImportProjectAnalysisSummary {
        progress: load_import_progress(context, project_id).await?,
        analyzed_chapters,
        outlined_chapters,
        generated_parts: completed_import_part_outlines(
            progress,
            needs_part_outlines,
            chapter_count,
        )?,
    })
}
