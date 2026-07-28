//! 章节结构化分析 runtime 窄 facade。

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use a4n_orchestrator::{
    OrchestratorContext,
    chapter_analysis::{ChapterAnalysisApplicationProgress, ChapterAnalysisHostPort},
    character::CharacterProfileSyncStageTracePort,
};
use a4n_prompt::rendering::PromptRenderer;

pub(crate) struct ChapterAnalysisRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> ChapterAnalysisRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn preflight_chapter_analysis_validated(
        &self,
        chapter_id: i64,
        require_saved_analysis: bool,
    ) -> Result<chapter::Model, AppError> {
        self.orchestrator
            .preflight_chapter_analysis_validated(chapter_id, require_saved_analysis)
            .await
    }

    pub(crate) async fn prepare_chapter_analysis_preview_validated(
        &self,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<
        (
            chapter::Model,
            Vec<a4n_orchestrator::chapter_analysis::ChapterAnalysisPreviewStep>,
        ),
        AppError,
    > {
        self.orchestrator
            .prepare_chapter_analysis_preview_validated(chapter_id, prompt_renderer)
            .await
    }

    pub(crate) async fn prepare_resume_chapter_analysis_preview_validated(
        &self,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<
        (
            chapter::Model,
            Vec<a4n_orchestrator::chapter_analysis::ChapterAnalysisPreviewStep>,
        ),
        AppError,
    > {
        self.orchestrator
            .prepare_resume_chapter_analysis_preview_validated(chapter_id, prompt_renderer)
            .await
    }

    pub(crate) async fn run_chapter_analysis_validated(
        &self,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<chapter::Model, AppError> {
        self.orchestrator
            .run_chapter_analysis_validated(
                chapter_id,
                prompt_renderer,
                host_port,
                progress,
                trace_port,
            )
            .await
    }

    pub(crate) async fn run_chapter_analysis_after_content_postprocess_validated(
        &self,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<chapter::Model, AppError> {
        self.orchestrator
            .run_chapter_analysis_after_content_postprocess_validated(
                chapter_id,
                prompt_renderer,
                host_port,
                progress,
                trace_port,
            )
            .await
    }

    pub(crate) async fn resume_chapter_analysis_sync_validated(
        &self,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<chapter::Model, AppError> {
        self.orchestrator
            .resume_chapter_analysis_sync_validated(
                chapter_id,
                prompt_renderer,
                host_port,
                progress,
                trace_port,
            )
            .await
    }

    pub(crate) async fn resume_chapter_analysis_workflow_run_validated(
        &self,
        run: &a4n_core::models::generation::workflow_run::Model,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<(), AppError> {
        self.orchestrator
            .resume_chapter_analysis_workflow_run_validated(
                run,
                prompt_renderer,
                host_port,
                trace_port,
            )
            .await
    }
}
