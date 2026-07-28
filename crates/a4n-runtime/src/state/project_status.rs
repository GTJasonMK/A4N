use a4n_core::{error::AppError, models::project::novel_project};
use a4n_db::project_reset::ProjectStatusRollbackOutcome;
use a4n_orchestrator::OrchestratorContext;

pub(crate) struct ProjectStatusRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> ProjectStatusRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn forward_project_status_record(
        &self,
        project_id: i64,
        target: &str,
    ) -> Result<novel_project::Model, AppError> {
        self.orchestrator
            .forward_project_status_record(project_id, target)
            .await
    }

    pub(crate) async fn rollback_project_status_record(
        &self,
        project_id: i64,
        target: &str,
    ) -> Result<ProjectStatusRollbackOutcome, AppError> {
        self.orchestrator
            .rollback_project_status_record(project_id, target)
            .await
    }
}
