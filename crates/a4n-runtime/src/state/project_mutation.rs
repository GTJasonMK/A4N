use a4n_core::{error::AppError, models::project::novel_project};
use a4n_orchestrator::{
    OrchestratorContext,
    chapter_content::ChapterDerivedCleanupHostPort,
    project::{CreateDerivedProjectParams, CreateFreeformVideoProjectParams},
};

pub(crate) struct ProjectMutationRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> ProjectMutationRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn create_project_validated(
        &self,
        user_id: i64,
        title: String,
        description: Option<String>,
        project_type: Option<String>,
    ) -> Result<novel_project::Model, AppError> {
        self.orchestrator
            .create_project_validated(user_id, title, description, project_type)
            .await
    }

    pub(crate) async fn run_import_text_project_validated(
        &self,
        host_port: &dyn ChapterDerivedCleanupHostPort,
        user_id: i64,
        root: serde_json::Value,
    ) -> Result<novel_project::Model, AppError> {
        self.orchestrator
            .run_import_text_project_validated(host_port, user_id, root)
            .await
    }

    pub(crate) async fn create_derived_project_validated(
        &self,
        owner_user_id: i64,
        params: CreateDerivedProjectParams,
    ) -> Result<novel_project::Model, AppError> {
        self.orchestrator
            .create_derived_project_validated(owner_user_id, params)
            .await
    }

    pub(crate) async fn create_freeform_video_project_validated(
        &self,
        user_id: i64,
        params: CreateFreeformVideoProjectParams,
    ) -> Result<novel_project::Model, AppError> {
        self.orchestrator
            .create_freeform_video_project_validated(user_id, params)
            .await
    }

    pub(crate) async fn update_project_validated(
        &self,
        project_id: i64,
        title: Option<String>,
        description: Option<String>,
    ) -> Result<novel_project::Model, AppError> {
        self.orchestrator
            .update_project_validated(project_id, title, description)
            .await
    }
}
