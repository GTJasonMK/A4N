use a4n_api_contract::import::ImportAnalysisStatus;
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;

use super::{
    ImportProjectAnalysisSummary, ProjectImportAnalysisProgress,
    chapter_outline::importable_chapters,
    progress::{
        finish_import_progress, initial_import_progress, load_import_progress_optional,
        resume_import_progress, save_failed_import_progress, save_import_progress,
        update_import_progress,
    },
};
use crate::OrchestratorContext;

pub(super) enum ImportAnalysisStageResult {
    Complete(usize),
    Cancelled(usize),
}

pub(super) struct ImportAnalysisRunPreparation {
    pub(super) chapters: Vec<chapter::Model>,
    pub(super) progress: ProjectImportAnalysisProgress,
}

pub(super) async fn save_import_stage(
    context: &OrchestratorContext,
    project_id: i64,
    progress: &mut ProjectImportAnalysisProgress,
    completed: usize,
    message: String,
) -> Result<(), AppError> {
    if super::progress::import_analysis_cancelled(context, project_id).await? {
        return Err(AppError::Cancelled);
    }
    update_import_progress(progress, completed, message)?;
    save_import_progress(context, project_id, progress).await
}

pub(super) async fn prepare_import_project_analysis(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<ImportAnalysisRunPreparation, AppError> {
    let (chapters_result, progress_result) = futures::join!(
        importable_chapters(context, project_id),
        load_import_progress_optional(context, project_id),
    );
    let chapters = chapters_result?;
    if chapters.is_empty() {
        return Err(AppError::InvalidState {
            current: "no_importable_chapters".to_string(),
            operation: "运行导入项目分析".to_string(),
        });
    }

    let progress = match progress_result? {
        Some(mut progress)
            if progress.stages.generating_analysis_data.total == chapters.len()
                && progress.status != ImportAnalysisStatus::Completed =>
        {
            resume_import_progress(&mut progress)?;
            progress
        }
        Some(_) | None => initial_import_progress(chapters.len())?,
    };
    save_import_progress(context, project_id, &progress).await?;

    Ok(ImportAnalysisRunPreparation { chapters, progress })
}

pub(super) async fn record_import_analysis_failure(
    context: &OrchestratorContext,
    project_id: i64,
    execution_error: &AppError,
) -> Result<(), AppError> {
    let execution_error = execution_error.to_string();
    save_failed_import_progress(context, project_id, &execution_error)
        .await
        .map_err(|progress_error| AppError::InvalidState {
            current: execution_error,
            operation: format!("记录导入失败进度失败: {progress_error}"),
        })
}

pub(super) async fn complete_import_project_analysis(
    context: &OrchestratorContext,
    project_id: i64,
    progress: &mut ProjectImportAnalysisProgress,
    analyzed_chapters: usize,
) -> Result<ImportProjectAnalysisSummary, AppError> {
    finish_import_progress(progress)?;
    save_import_progress(context, project_id, progress).await?;
    context
        .sync_project_status_from_artifacts(project_id)
        .await?;

    Ok(ImportProjectAnalysisSummary {
        progress: progress.clone(),
        analyzed_chapters,
    })
}
