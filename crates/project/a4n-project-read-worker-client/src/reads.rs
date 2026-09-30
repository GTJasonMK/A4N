use a4n_api_contract::project::{
    PaginatedProjects, ProjectListInput, ProjectReadApplicationPort, ProjectSummary,
};
use a4n_error::AppError;
use a4n_project_read_worker_contract::{
    PROJECT_READ_EXISTS_OPERATION, PROJECT_READ_IDS_OPERATION, PROJECT_READ_LIST_OPERATION,
    PROJECT_READ_SUMMARY_OPERATION, ProjectIdInput, ProjectServiceInput,
};

use super::ProjectReadWorkerClient;

#[async_trait::async_trait]
impl ProjectReadApplicationPort for ProjectReadWorkerClient {
    async fn summary(&self, user_id: i64, project_id: i64) -> Result<ProjectSummary, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("project_id 必须大于 0".into()));
        }
        self.command(
            Self::user_principal(user_id)?,
            PROJECT_READ_SUMMARY_OPERATION,
            Some(project_id),
            ProjectIdInput { project_id },
        )
        .await
    }

    async fn list(
        &self,
        user_id: i64,
        input: ProjectListInput,
    ) -> Result<PaginatedProjects, AppError> {
        self.command(
            Self::user_principal(user_id)?,
            PROJECT_READ_LIST_OPERATION,
            None,
            input,
        )
        .await
    }

    async fn all_project_ids(&self) -> Result<Vec<i64>, AppError> {
        self.command(
            Self::service_principal(),
            PROJECT_READ_IDS_OPERATION,
            None,
            ProjectServiceInput {},
        )
        .await
    }

    async fn exists(&self, project_id: i64) -> Result<bool, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("project_id 必须大于 0".into()));
        }
        self.command(
            Self::service_principal(),
            PROJECT_READ_EXISTS_OPERATION,
            Some(project_id),
            ProjectIdInput { project_id },
        )
        .await
    }
}
