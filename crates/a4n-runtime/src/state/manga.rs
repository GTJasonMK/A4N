//! 漫画领域 runtime 窄 facade。

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_manga_prompt;
use a4n_orchestrator::{
    OrchestratorContext,
    manga::{
        BindReferenceImageInput, MangaPipelineApplicationInput,
        MangaPipelinePreviewApplicationInput, MangaPipelineRequestPreviewStep,
        MangaPipelineStatusSummary, MangaPromptDownstreamCleanupPort,
        OptimizeMangaPromptRecordInput, UpdateMangaPromptInput,
    },
};

pub(crate) struct MangaRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> MangaRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn list_manga_panels_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        self.orchestrator
            .list_manga_panels_validated(user_id, project_id, chapter_id)
            .await
    }

    pub(crate) async fn bind_manga_reference_image_validated(
        &self,
        user_id: i64,
        input: BindReferenceImageInput,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        self.orchestrator
            .bind_manga_reference_image_validated(user_id, input, cleanup)
            .await
    }

    pub(crate) async fn update_manga_prompt_validated(
        &self,
        user_id: i64,
        input: UpdateMangaPromptInput,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        self.orchestrator
            .update_manga_prompt_validated(user_id, input, cleanup)
            .await
    }

    pub(crate) async fn delete_manga_by_chapter_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_manga_by_chapter_validated(user_id, project_id, chapter_id)
            .await
    }

    pub(crate) async fn delete_manga_prompt_validated(
        &self,
        user_id: i64,
        prompt_id: i64,
        downstream_strategy: a4n_core::dependency_conflict::DownstreamConflictStrategy,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_manga_prompt_validated(user_id, prompt_id, downstream_strategy, cleanup)
            .await
    }

    pub(crate) async fn get_manga_pipeline_status_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<MangaPipelineStatusSummary, AppError> {
        self.orchestrator
            .get_manga_pipeline_status_validated(user_id, project_id, chapter_id)
            .await
    }

    pub(crate) async fn cancel_manga_pipeline_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<i64, AppError> {
        self.orchestrator
            .cancel_manga_pipeline_validated(user_id, project_id, chapter_id)
            .await
    }

    pub(crate) async fn optimize_manga_prompt_record_validated(
        &self,
        user_id: i64,
        input: OptimizeMangaPromptRecordInput,
        prompt_renderer: &dyn a4n_prompt::rendering::PromptRenderer,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        self.orchestrator
            .optimize_manga_prompt_record_validated(user_id, input, prompt_renderer, cleanup)
            .await
    }

    pub(crate) async fn preflight_manga_pipeline_validated(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<(), AppError> {
        self.orchestrator
            .preflight_manga_pipeline_validated(project_id, chapter_number)
            .await
    }

    pub(crate) async fn run_manga_pipeline_validated(
        &self,
        input: MangaPipelineApplicationInput<'_>,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        self.orchestrator.run_manga_pipeline_validated(input).await
    }

    pub(crate) async fn prepare_manga_pipeline_preview_validated(
        &self,
        input: MangaPipelinePreviewApplicationInput<'_>,
    ) -> Result<Vec<MangaPipelineRequestPreviewStep>, AppError> {
        self.orchestrator
            .prepare_manga_pipeline_preview_validated(input)
            .await
    }
}
