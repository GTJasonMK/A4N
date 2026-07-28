use a4n_core::{
    error::AppError,
    models::chapter::{chapter_outline, part_outline},
};
use a4n_db::project::ProjectChapterOutlineUpdateInput;
use a4n_orchestrator::OrchestratorContext;

pub(crate) struct ProjectOutlineRecordsRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> ProjectOutlineRecordsRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn chapter_outline_by_id(
        &self,
        outline_id: i64,
    ) -> Result<Option<chapter_outline::Model>, AppError> {
        self.orchestrator.chapter_outline_by_id(outline_id).await
    }

    pub(crate) async fn update_chapter_outline_validated(
        &self,
        input: ProjectChapterOutlineUpdateInput,
    ) -> Result<chapter_outline::Model, AppError> {
        self.orchestrator
            .update_chapter_outline_validated(input)
            .await
    }

    pub(crate) async fn chapter_outlines_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        self.orchestrator
            .chapter_outlines_for_project(project_id)
            .await
    }

    pub(crate) async fn part_outlines_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<part_outline::Model>, AppError> {
        self.orchestrator
            .part_outlines_for_project(project_id)
            .await
    }

    pub(crate) async fn part_outline_by_id(
        &self,
        part_id: i64,
    ) -> Result<Option<part_outline::Model>, AppError> {
        self.orchestrator.part_outline_by_id(part_id).await
    }
}
