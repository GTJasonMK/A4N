use a4n_core::{error::AppError, models::chapter::chapter};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

use crate::{
    OrchestratorContext,
    chapter_analysis::{
        ChapterAnalysisApplicationProgress, ChapterAnalysisHostPort, ChapterAnalysisPreviewStep,
    },
    character::CharacterProfileSyncStageTracePort,
};

use super::{
    ImportProjectAnalysisSummary,
    chapter_outline::importable_chapters,
    lifecycle::cancelled_import_analysis_summary,
    progress::{STAGE_ANALYSIS, import_analysis_cancelled},
    summary::completed_import_stage,
    workflow::{
        ImportAnalysisStageResult, PreparedImportAnalysisRun, complete_import_project_analysis,
        prepare_import_project_analysis, record_import_analysis_failure, save_import_stage,
    },
};

impl OrchestratorContext {
    pub async fn run_import_project_analysis_validated(
        &self,
        project_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        chapter_analysis_host_port: &dyn ChapterAnalysisHostPort,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ImportProjectAnalysisSummary, AppError> {
        let mut prepared = match prepare_import_project_analysis(self, project_id)
            .await
            .map_err(import_analysis_error)?
        {
            PreparedImportAnalysisRun::Completed(summary) => return Ok(summary),
            PreparedImportAnalysisRun::Ready(prepared) => prepared,
        };

        let result = self
            .execute_import_project_analysis(
                project_id,
                &prepared.chapters,
                &mut prepared.progress,
                prompt_renderer,
                chapter_analysis_host_port,
                character_trace_port,
            )
            .await;
        match result {
            Ok(summary) => Ok(summary),
            Err(AppError::Cancelled) => Err(AppError::Cancelled),
            Err(error) => {
                let error_text = error.to_string();
                record_import_analysis_failure(self, project_id, &error_text)
                    .await
                    .map_err(import_analysis_error)?;
                Err(error)
            }
        }
    }

    pub async fn prepare_import_project_analysis_preview_validated(
        &self,
        project_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<ChapterAnalysisPreviewStep>, AppError> {
        let chapters = importable_chapters(self, project_id)
            .await
            .map_err(import_analysis_error)?;
        if chapters.is_empty() {
            return Err(AppError::InvalidState {
                current: "no_importable_chapters".to_string(),
                operation: "预览导入项目分析请求".to_string(),
            });
        }
        let mut steps = Vec::with_capacity(chapters.len() * 4 + 1);
        for chapter in chapters {
            for request in self
                .build_chapter_analysis_requests(chapter.id, prompt_renderer)
                .await?
            {
                steps.push(ChapterAnalysisPreviewStep {
                    key: format!("import_chapter_analysis_{}_{}", chapter.id, request.key),
                    title: format!("第 {} 章{}", chapter.chapter_number, request.title),
                    prompt_name: Some(request.prompt_name),
                    status: "ready".to_string(),
                    note: Some("导入项目分析的分析阶段会按顺序发送该请求".to_string()),
                    request: Some(request.request),
                });
            }
        }
        steps.push(ChapterAnalysisPreviewStep {
            key: "import_analysis_character_sync".to_string(),
            title: "导入章节角色发现和角色同步请求".to_string(),
            prompt_name: Some("character_discovery".to_string()),
            status: "blocked".to_string(),
            note: Some(
                "这些请求依赖对应章节结构化分析输出，运行前无法提前构造真实请求".to_string(),
            ),
            request: None,
        });
        Ok(steps)
    }

    async fn execute_import_project_analysis(
        &self,
        project_id: i64,
        chapters: &[chapter::Model],
        progress: &mut Value,
        prompt_renderer: &dyn PromptRenderer,
        chapter_analysis_host_port: &dyn ChapterAnalysisHostPort,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ImportProjectAnalysisSummary, AppError> {
        let total_chapters = chapters.len();
        let mut analyzed = completed_import_stage(progress, STAGE_ANALYSIS, total_chapters)
            .map_err(import_analysis_error)?;
        if analyzed < total_chapters {
            match self
                .run_import_analysis_chapter_stage(
                    project_id,
                    chapters,
                    analyzed,
                    progress,
                    prompt_renderer,
                    chapter_analysis_host_port,
                    character_trace_port,
                )
                .await?
            {
                ImportAnalysisStageResult::Complete(value) => analyzed = value,
                ImportAnalysisStageResult::Cancelled(value) => {
                    return cancelled_import_analysis_summary(self, project_id, value)
                        .await
                        .map_err(import_analysis_error);
                }
            }
        } else {
            analyzed = total_chapters;
        }

        if import_analysis_cancelled(self, project_id)
            .await
            .map_err(import_analysis_error)?
        {
            return cancelled_import_analysis_summary(self, project_id, analyzed)
                .await
                .map_err(import_analysis_error);
        }
        complete_import_project_analysis(self, project_id, progress, analyzed)
            .await
            .map_err(import_analysis_error)
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_import_analysis_chapter_stage(
        &self,
        project_id: i64,
        chapters: &[chapter::Model],
        mut analyzed: usize,
        progress: &mut Value,
        prompt_renderer: &dyn PromptRenderer,
        chapter_analysis_host_port: &dyn ChapterAnalysisHostPort,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ImportAnalysisStageResult, AppError> {
        let total = chapters.len();
        save_import_stage(
            self,
            project_id,
            progress,
            STAGE_ANALYSIS,
            analyzed,
            total,
            format!(
                "正在分析并同步第{}章资料",
                chapters[analyzed].chapter_number
            ),
        )
        .await
        .map_err(import_analysis_error)?;
        for chapter in chapters.iter().skip(analyzed) {
            if import_analysis_cancelled(self, project_id)
                .await
                .map_err(import_analysis_error)?
            {
                return Ok(ImportAnalysisStageResult::Cancelled(analyzed));
            }
            save_import_stage(
                self,
                project_id,
                progress,
                STAGE_ANALYSIS,
                analyzed,
                total,
                format!("正在分析并同步第{}章资料", chapter.chapter_number),
            )
            .await
            .map_err(import_analysis_error)?;
            self.run_chapter_analysis_validated(
                chapter.id,
                prompt_renderer,
                chapter_analysis_host_port,
                ChapterAnalysisApplicationProgress::default(),
                character_trace_port,
            )
            .await
            .map_err(|error| import_chapter_analysis_error(error, chapter.chapter_number))?;
            analyzed += 1;
            save_import_stage(
                self,
                project_id,
                progress,
                STAGE_ANALYSIS,
                analyzed,
                total,
                format!("已完成第{}章分析", chapter.chapter_number),
            )
            .await
            .map_err(import_analysis_error)?;
        }
        save_import_stage(
            self,
            project_id,
            progress,
            STAGE_ANALYSIS,
            total,
            total,
            "逐章分析与资料同步完成".to_owned(),
        )
        .await
        .map_err(import_analysis_error)?;
        Ok(ImportAnalysisStageResult::Complete(total))
    }
}

fn import_analysis_error(error: impl ToString) -> AppError {
    AppError::InvalidState {
        current: "import_project_analysis".to_string(),
        operation: error.to_string(),
    }
}

fn import_chapter_analysis_error(error: AppError, chapter_number: i32) -> AppError {
    match error {
        AppError::Cancelled => AppError::Cancelled,
        error => AppError::InvalidState {
            current: format!("import_chapter_{chapter_number}"),
            operation: format!("结构化分析失败: {error}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imported_chapter_analysis_preserves_cancellation() {
        assert!(matches!(
            import_chapter_analysis_error(AppError::Cancelled, 1),
            AppError::Cancelled
        ));
    }
}
