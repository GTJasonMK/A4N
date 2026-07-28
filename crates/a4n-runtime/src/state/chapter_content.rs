use a4n_core::error::AppError;
use a4n_orchestrator::{
    OrchestratorContext,
    chapter_content::{ChapterDerivedCleanupHostPort, ChapterOutlineCleanupHostPort},
};

pub(crate) struct ChapterContentRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> ChapterContentRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn chapters_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_core::models::chapter::chapter::Model>, AppError> {
        self.orchestrator.chapters_for_project(project_id).await
    }

    pub(crate) async fn preflight_chapter_content_change_validated(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<(), AppError> {
        self.orchestrator
            .preflight_chapter_content_change_validated(project_id, chapter_number)
            .await
    }

    pub(crate) async fn postprocess_chapter_after_content_change_validated(
        &self,
        host_port: &dyn ChapterDerivedCleanupHostPort,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .postprocess_chapter_after_content_change_validated(host_port, chapter_id)
            .await
    }

    pub(crate) async fn delete_latest_chapter_outlines_validated(
        &self,
        host_port: &dyn ChapterOutlineCleanupHostPort,
        project_id: i64,
        count: u64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_latest_chapter_outlines_validated(host_port, project_id, count)
            .await
    }

    pub(crate) async fn import_chapter_validated(
        &self,
        user_id: i64,
        host_port: &dyn ChapterDerivedCleanupHostPort,
        project_id: i64,
        chapter_number: i32,
        content: String,
        title: Option<String>,
    ) -> Result<(a4n_core::models::chapter::chapter::Model, i32), AppError> {
        self.orchestrator
            .import_chapter_validated(
                user_id,
                host_port,
                project_id,
                chapter_number,
                content,
                title,
            )
            .await
    }

    pub(crate) async fn update_chapter_content_validated(
        &self,
        user_id: i64,
        host_port: &dyn ChapterDerivedCleanupHostPort,
        chapter_id: i64,
        content: String,
        title: Option<String>,
    ) -> Result<a4n_core::models::chapter::chapter::Model, AppError> {
        self.orchestrator
            .update_chapter_content_validated(user_id, host_port, chapter_id, content, title)
            .await
    }
}
