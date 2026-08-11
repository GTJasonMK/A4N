mod completion;

use a4n_core::models::chapter::chapter;
use serde_json::Value;

use super::{
    ImportProjectAnalysisSummary,
    chapter_outline::importable_chapters,
    progress::{
        STAGE_ANALYSIS, import_progress_resume_stage, import_progress_totals_match,
        initial_import_progress, load_import_progress_optional, resume_import_progress,
        save_failed_import_progress, save_import_progress, update_import_progress_stage,
    },
    summary::completed_import_stage,
};
use crate::OrchestratorContext;

pub use completion::complete_import_project_analysis;

pub enum ImportAnalysisStageResult {
    Complete(usize),
    Cancelled(usize),
}

pub struct ImportAnalysisRunPreparation {
    pub chapters: Vec<chapter::Model>,
    pub progress: Value,
}

pub enum PreparedImportAnalysisRun {
    Completed(ImportProjectAnalysisSummary),
    Ready(ImportAnalysisRunPreparation),
}

pub async fn save_import_stage(
    context: &OrchestratorContext,
    project_id: i64,
    progress: &mut Value,
    stage: &str,
    completed: usize,
    total: usize,
    message: String,
) -> Result<(), String> {
    update_import_progress_stage(progress, stage, completed, total, message)?;
    save_import_progress(context, project_id, progress)
        .await
        .map(|_| ())
}

pub async fn prepare_import_project_analysis(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<PreparedImportAnalysisRun, String> {
    let chapters = importable_chapters(context, project_id).await?;
    if chapters.is_empty() {
        return Err("项目没有可分析的章节".to_string());
    }

    let mut progress =
        if let Some(saved_progress) = load_import_progress_optional(context, project_id).await? {
            saved_progress
        } else {
            initial_import_progress(chapters.len())
        };
    if import_progress_totals_match(&progress, chapters.len()) {
        if progress.get("status").and_then(Value::as_str) == Some("completed") {
            let analyzed_chapters =
                completed_import_stage(&progress, STAGE_ANALYSIS, chapters.len())?;
            return Ok(PreparedImportAnalysisRun::Completed(
                ImportProjectAnalysisSummary {
                    progress,
                    analyzed_chapters,
                },
            ));
        }
        if let Some(stage) = import_progress_resume_stage(&progress) {
            resume_import_progress(&mut progress, &stage)?;
        } else {
            progress = initial_import_progress(chapters.len());
        }
    } else {
        progress = initial_import_progress(chapters.len());
    }
    save_import_progress(context, project_id, &progress).await?;

    Ok(PreparedImportAnalysisRun::Ready(
        ImportAnalysisRunPreparation { chapters, progress },
    ))
}

pub async fn record_import_analysis_failure(
    context: &OrchestratorContext,
    project_id: i64,
    execution_error: &str,
) -> Result<(), String> {
    save_failed_import_progress(context, project_id, execution_error)
        .await
        .map_err(|progress_error| {
            format!("{execution_error}; 记录导入失败进度失败: {progress_error}")
        })
}
