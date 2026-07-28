//! 导入分析 runtime 窄 facade。

use a4n_core::{error::AppError, models::project::import_review_queue};
use a4n_orchestrator::{
    OrchestratorContext,
    chapter_analysis::{ChapterAnalysisHostPort, ChapterAnalysisPreviewStep},
    character::CharacterProfileSyncStageTracePort,
    import_analysis::{
        ImportProjectAnalysisSummary, StartImportAnalysisInput,
        ingest::create_import_analysis_reviews,
        lifecycle::{cancel_import_analysis, project_import_analysis_progress},
    },
};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

pub(crate) struct ImportAnalysisRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> ImportAnalysisRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn run_import_project_analysis_validated(
        &self,
        project_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        chapter_analysis_host_port: &dyn ChapterAnalysisHostPort,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ImportProjectAnalysisSummary, AppError> {
        self.orchestrator
            .run_import_project_analysis_validated(
                project_id,
                prompt_renderer,
                chapter_analysis_host_port,
                character_trace_port,
            )
            .await
    }

    pub(crate) async fn prepare_import_project_analysis_preview_validated(
        &self,
        project_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<ChapterAnalysisPreviewStep>, AppError> {
        self.orchestrator
            .prepare_import_project_analysis_preview_validated(project_id, prompt_renderer)
            .await
    }

    pub(crate) async fn project_import_analysis_progress(
        &self,
        project_id: i64,
    ) -> Result<Option<Value>, String> {
        project_import_analysis_progress(self.orchestrator, project_id).await
    }

    pub(crate) async fn cancel_import_analysis(&self, project_id: i64) -> Result<u64, String> {
        cancel_import_analysis(self.orchestrator, project_id).await
    }

    pub(crate) async fn create_import_analysis_reviews(
        &self,
        project_id: i64,
        input: StartImportAnalysisInput,
    ) -> Result<Vec<import_review_queue::Model>, String> {
        create_import_analysis_reviews(self.orchestrator, project_id, input).await
    }
}
