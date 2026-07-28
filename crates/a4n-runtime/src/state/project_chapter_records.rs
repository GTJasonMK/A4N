use a4n_core::{error::AppError, models::chapter::chapter};
use a4n_orchestrator::OrchestratorContext;

pub(crate) struct ProjectChapterRecordsRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> ProjectChapterRecordsRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn chapter_by_id(
        &self,
        chapter_id: i64,
    ) -> Result<Option<chapter::Model>, AppError> {
        self.orchestrator.chapter_by_id(chapter_id).await
    }

    pub(crate) async fn validate_chapter_generation_readiness(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<(), String> {
        self.orchestrator
            .validate_chapter_generation_readiness(project_id, chapter_number)
            .await
    }
}
