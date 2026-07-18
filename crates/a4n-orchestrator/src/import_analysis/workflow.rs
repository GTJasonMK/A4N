mod completion;
mod outlines;
mod parts;

use a4n_core::models::{chapter::chapter, project::novel_project};
use a4n_db::project::needs_part_outlines_for_chapters;
use serde_json::Value;

use super::{
    ImportProjectAnalysisSummary,
    chapter_outline::importable_chapters,
    progress::{
        STAGE_ANALYSIS, STAGE_OUTLINES, import_progress_resume_stage, import_progress_totals_match,
        initial_import_progress, load_import_progress_optional, resume_import_progress,
        save_failed_import_progress, save_import_progress, update_import_progress_stage,
    },
    summary::{completed_import_part_outlines, completed_import_stage},
};
use crate::OrchestratorContext;

pub use completion::complete_import_project_analysis;
pub use outlines::run_import_analysis_outlines;
pub use parts::run_import_analysis_parts;

pub enum ImportAnalysisStageResult {
    Complete(usize),
    Cancelled(usize),
}

pub struct ImportAnalysisRunPreparation {
    pub project: novel_project::Model,
    pub chapters: Vec<chapter::Model>,
    pub needs_part_outlines: bool,
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
    let project = context
        .project_by_id(project_id)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("项目不存在: project_id={project_id}"))?;
    let chapters = importable_chapters(context, project_id).await?;
    if chapters.is_empty() {
        return Err("项目没有可分析的章节".to_string());
    }

    let chapter_count =
        i32::try_from(chapters.len()).map_err(|_| "导入分析章节数量超出 i32 范围".to_string())?;
    let needs_part_outlines = needs_part_outlines_for_chapters(chapter_count);
    let mut progress =
        if let Some(saved_progress) = load_import_progress_optional(context, project_id).await? {
            saved_progress
        } else {
            initial_import_progress(chapters.len(), needs_part_outlines)
        };
    if import_progress_totals_match(&progress, chapters.len(), needs_part_outlines) {
        if progress.get("status").and_then(Value::as_str) == Some("completed") {
            let analyzed_chapters =
                completed_import_stage(&progress, STAGE_ANALYSIS, chapters.len())?;
            let outlined_chapters =
                completed_import_stage(&progress, STAGE_OUTLINES, chapters.len())?;
            let generated_parts =
                completed_import_part_outlines(&progress, needs_part_outlines, chapters.len())?;
            return Ok(PreparedImportAnalysisRun::Completed(
                ImportProjectAnalysisSummary {
                    progress,
                    analyzed_chapters,
                    outlined_chapters,
                    generated_parts,
                },
            ));
        }
        if let Some(stage) = import_progress_resume_stage(&progress, needs_part_outlines) {
            resume_import_progress(&mut progress, &stage)?;
        } else {
            progress = initial_import_progress(chapters.len(), needs_part_outlines);
        }
    } else {
        progress = initial_import_progress(chapters.len(), needs_part_outlines);
    }
    save_import_progress(context, project_id, &progress).await?;

    Ok(PreparedImportAnalysisRun::Ready(
        ImportAnalysisRunPreparation {
            project,
            chapters,
            needs_part_outlines,
            progress,
        },
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
