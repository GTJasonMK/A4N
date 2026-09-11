use super::{
    ImportProjectAnalysisSummary, ProjectImportAnalysisProgress,
    progress::{cancel_import_progress, load_import_progress, load_import_progress_optional},
};
use crate::OrchestratorContext;
use a4n_error::AppError;

pub async fn project_import_analysis_progress(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<Option<ProjectImportAnalysisProgress>, AppError> {
    load_import_progress_optional(context, project_id).await
}

pub async fn cancel_import_analysis(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<(), AppError> {
    cancel_import_progress(context, project_id).await
}

pub(super) async fn cancelled_import_analysis_summary(
    context: &OrchestratorContext,
    project_id: i64,
    analyzed_chapters: usize,
) -> Result<ImportProjectAnalysisSummary, AppError> {
    Ok(ImportProjectAnalysisSummary {
        progress: load_import_progress(context, project_id).await?,
        analyzed_chapters,
    })
}
