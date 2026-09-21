use a4n_chapter_analysis_host_contract::ChapterAnalysisRequestStep;
use a4n_db_models::models::chapter::chapter;
use a4n_error::AppError;
use a4n_import_analysis_host_contract::{
    ImportAnalysisGenerationApplicationPort, ImportAnalysisPreviewApplicationPort,
};
use a4n_prompt::rendering::PromptRenderer;
use async_trait::async_trait;

use crate::{
    OrchestratorContext,
    chapter_analysis::{
        ChapterAnalysisApplicationProgress, ChapterAnalysisHostPort,
        build_chapter_analysis_initial_request,
    },
    character::CharacterProfileSyncStageTracePort,
};

use super::{
    ImportProjectAnalysisSummary, ProjectImportAnalysisProgress,
    chapter_outline::importable_chapters,
    lifecycle::cancelled_import_analysis_summary,
    progress::{completed_import_chapters, import_analysis_cancelled},
    workflow::{
        ImportAnalysisStageResult, complete_import_project_analysis,
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
        let mut prepared = prepare_import_project_analysis(self, project_id).await?;

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
            Err(_) if import_analysis_cancelled(self, project_id).await? => {
                cancelled_import_analysis_summary(
                    self,
                    project_id,
                    completed_import_chapters(&prepared.progress)?,
                )
                .await
            }
            Err(AppError::Cancelled) => Err(AppError::Cancelled),
            Err(error) => {
                record_import_analysis_failure(self, project_id, &error).await?;
                Err(error)
            }
        }
    }

    pub async fn prepare_import_project_analysis_preview_validated(
        &self,
        project_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<ChapterAnalysisRequestStep>, AppError> {
        let chapters = importable_chapters(self, project_id).await?;
        if chapters.is_empty() {
            return Err(AppError::InvalidState {
                current: "no_importable_chapters".to_string(),
                operation: "预览导入项目分析请求".to_string(),
            });
        }
        let mut steps = Vec::with_capacity(chapters.len());
        for chapter in chapters {
            let mut request =
                build_chapter_analysis_initial_request(&chapter, prompt_renderer).await?;
            request.key = format!("import_chapter_analysis_{}_{}", chapter.id, request.key);
            request.title = format!("第 {} 章{}", chapter.chapter_number, request.title);
            steps.push(request);
        }
        Ok(steps)
    }

    async fn execute_import_project_analysis(
        &self,
        project_id: i64,
        chapters: &[chapter::Model],
        progress: &mut ProjectImportAnalysisProgress,
        prompt_renderer: &dyn PromptRenderer,
        chapter_analysis_host_port: &dyn ChapterAnalysisHostPort,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ImportProjectAnalysisSummary, AppError> {
        let total_chapters = chapters.len();
        let mut analyzed = completed_import_chapters(progress)?;
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
                    return cancelled_import_analysis_summary(self, project_id, value).await;
                }
            }
        }

        if import_analysis_cancelled(self, project_id).await? {
            return cancelled_import_analysis_summary(self, project_id, analyzed).await;
        }
        complete_import_project_analysis(self, project_id, progress, analyzed).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_import_analysis_chapter_stage(
        &self,
        project_id: i64,
        chapters: &[chapter::Model],
        mut analyzed: usize,
        progress: &mut ProjectImportAnalysisProgress,
        prompt_renderer: &dyn PromptRenderer,
        chapter_analysis_host_port: &dyn ChapterAnalysisHostPort,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ImportAnalysisStageResult, AppError> {
        let total = chapters.len();
        save_import_stage(
            self,
            project_id,
            progress,
            analyzed,
            format!(
                "正在分析并同步第{}章资料",
                chapters[analyzed].chapter_number
            ),
        )
        .await?;
        for chapter in chapters.iter().skip(analyzed) {
            if import_analysis_cancelled(self, project_id).await? {
                return Ok(ImportAnalysisStageResult::Cancelled(analyzed));
            }
            save_import_stage(
                self,
                project_id,
                progress,
                analyzed,
                format!("正在分析并同步第{}章资料", chapter.chapter_number),
            )
            .await?;
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
                analyzed,
                format!("已完成第{}章分析", chapter.chapter_number),
            )
            .await?;
        }
        save_import_stage(
            self,
            project_id,
            progress,
            total,
            "逐章分析与资料同步完成".to_owned(),
        )
        .await?;
        Ok(ImportAnalysisStageResult::Complete(total))
    }
}

#[async_trait]
impl a4n_api_contract::import::ImportAnalysisProgressApplicationPort for OrchestratorContext {
    async fn progress(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Option<ProjectImportAnalysisProgress>, AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        super::lifecycle::project_import_analysis_progress(self, project_id).await
    }
}

#[async_trait]
impl ImportAnalysisGenerationApplicationPort for OrchestratorContext {
    async fn run(
        &self,
        user_id: i64,
        project_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        chapter_analysis_host_port: &dyn ChapterAnalysisHostPort,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ImportProjectAnalysisSummary, AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.run_import_project_analysis_validated(
            project_id,
            prompt_renderer,
            chapter_analysis_host_port,
            character_trace_port,
        )
        .await
    }
}

#[async_trait]
impl a4n_api_contract::import::ImportAnalysisCancellationApplicationPort for OrchestratorContext {
    async fn cancel(&self, user_id: i64, project_id: i64) -> Result<(), AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        super::lifecycle::cancel_import_analysis(self, project_id).await
    }
}

#[async_trait]
impl ImportAnalysisPreviewApplicationPort for OrchestratorContext {
    async fn preview(
        &self,
        user_id: i64,
        project_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<ChapterAnalysisRequestStep>, AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.prepare_import_project_analysis_preview_validated(project_id, prompt_renderer)
            .await
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
