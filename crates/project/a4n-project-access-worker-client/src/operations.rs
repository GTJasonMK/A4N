use a4n_api_contract::collaboration::{
    ProjectAccessApplicationPort, ProjectAccessRole, ProjectCollaborationRootSummary,
};
use a4n_error::AppError;
use a4n_project_access_worker_contract::{
    PROJECT_ACCESS_FILTER_OPERATION, PROJECT_ACCESS_ROOT_OPERATION,
    PROJECT_ACCESS_VERIFY_BATCH_OPERATION, PROJECT_ACCESS_VERIFY_OPERATION,
    ProjectAccessBatchInput, ProjectAccessInput, ProjectAccessRootInput,
};

use super::ProjectAccessWorkerClient;

#[async_trait::async_trait]
impl ProjectAccessApplicationPort for ProjectAccessWorkerClient {
    async fn verify_permission(
        &self,
        user_id: i64,
        project_id: i64,
        required: ProjectAccessRole,
    ) -> Result<ProjectAccessRole, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("project_id 必须大于 0".into()));
        }
        self.command(
            a4n_worker_client::user_principal(user_id)?,
            Some(project_id),
            PROJECT_ACCESS_VERIFY_OPERATION,
            ProjectAccessInput {
                project_id,
                required,
            },
        )
        .await
    }

    async fn verify_permissions(
        &self,
        user_id: i64,
        project_ids: &[i64],
        required: ProjectAccessRole,
    ) -> Result<(), AppError> {
        self.command(
            a4n_worker_client::user_principal(user_id)?,
            None,
            PROJECT_ACCESS_VERIFY_BATCH_OPERATION,
            ProjectAccessBatchInput {
                project_ids: project_ids.to_vec(),
                required,
            },
        )
        .await
    }

    async fn project_ids_with_permission(
        &self,
        user_id: i64,
        project_ids: &[i64],
        required: ProjectAccessRole,
    ) -> Result<Vec<i64>, AppError> {
        self.command(
            a4n_worker_client::user_principal(user_id)?,
            None,
            PROJECT_ACCESS_FILTER_OPERATION,
            ProjectAccessBatchInput {
                project_ids: project_ids.to_vec(),
                required,
            },
        )
        .await
    }

    async fn collaboration_root(
        &self,
        project_id: i64,
    ) -> Result<ProjectCollaborationRootSummary, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("project_id 必须大于 0".into()));
        }
        self.command(
            Self::service_principal(),
            Some(project_id),
            PROJECT_ACCESS_ROOT_OPERATION,
            ProjectAccessRootInput { project_id },
        )
        .await
    }
}
