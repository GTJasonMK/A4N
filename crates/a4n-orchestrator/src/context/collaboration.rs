use a4n_db::{DbHandle, collaboration::CollaborationDbError};
use a4n_error::AppError;

use super::OrchestratorContext;
use crate::collaboration::{
    CollaborationRootProject, ProjectAccessInfo, ProjectAccessInfoQuery, ProjectAccessRole,
};

fn collaboration_db_error_to_app_error(error: CollaborationDbError) -> AppError {
    match error {
        CollaborationDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        CollaborationDbError::NotFound(message) => AppError::NotFound(message),
        CollaborationDbError::Forbidden(message) => AppError::Forbidden(message),
        CollaborationDbError::InvalidState(message) => AppError::InvalidState {
            current: message,
            operation: "执行项目协作操作".to_string(),
        },
        CollaborationDbError::Internal(message) => AppError::Internal(anyhow::anyhow!(message)),
        CollaborationDbError::Database(error) => AppError::Database(error),
    }
}

#[derive(Clone)]
pub(super) struct CollaborationApplication {
    database: DbHandle,
}

impl CollaborationApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn project_access_info(
        &self,
        query: ProjectAccessInfoQuery,
        user_id: i64,
    ) -> Result<ProjectAccessInfo, AppError> {
        a4n_db::collaboration::project_access_info(self.database.connection(), query, user_id)
            .await
            .map_err(collaboration_db_error_to_app_error)
    }

    async fn collaboration_root_project(
        &self,
        project_id: i64,
    ) -> Result<CollaborationRootProject, AppError> {
        a4n_db::collaboration::resolve_root_project(self.database.connection(), project_id)
            .await
            .map_err(collaboration_db_error_to_app_error)
    }

    async fn verify_project_permission(
        &self,
        project_id: i64,
        user_id: i64,
        required: ProjectAccessRole,
    ) -> Result<ProjectAccessRole, AppError> {
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id,
            required,
        )
        .await
        .map_err(collaboration_db_error_to_app_error)
    }

    async fn verify_project_permissions(
        &self,
        project_ids: &[i64],
        user_id: i64,
        required: ProjectAccessRole,
    ) -> Result<(), AppError> {
        a4n_db::collaboration::verify_project_permissions(
            self.database.connection(),
            project_ids,
            user_id,
            required,
        )
        .await
        .map_err(collaboration_db_error_to_app_error)
    }

    async fn project_ids_with_permission(
        &self,
        project_ids: &[i64],
        user_id: i64,
        required: ProjectAccessRole,
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::collaboration::project_ids_with_permission(
            self.database.connection(),
            project_ids,
            user_id,
            required,
        )
        .await
        .map_err(collaboration_db_error_to_app_error)
    }
}

impl OrchestratorContext {
    pub(crate) async fn project_access_info(
        &self,
        query: ProjectAccessInfoQuery,
        user_id: i64,
    ) -> Result<ProjectAccessInfo, AppError> {
        self.collaboration.project_access_info(query, user_id).await
    }

    pub(crate) async fn collaboration_root_project(
        &self,
        project_id: i64,
    ) -> Result<CollaborationRootProject, AppError> {
        self.collaboration
            .collaboration_root_project(project_id)
            .await
    }

    pub(crate) async fn verify_project_permission(
        &self,
        project_id: i64,
        user_id: i64,
        required: ProjectAccessRole,
    ) -> Result<ProjectAccessRole, AppError> {
        self.collaboration
            .verify_project_permission(project_id, user_id, required)
            .await
    }

    pub(crate) async fn verify_project_permissions(
        &self,
        project_ids: &[i64],
        user_id: i64,
        required: ProjectAccessRole,
    ) -> Result<(), AppError> {
        self.collaboration
            .verify_project_permissions(project_ids, user_id, required)
            .await
    }

    pub(crate) async fn project_ids_with_permission(
        &self,
        project_ids: &[i64],
        user_id: i64,
        required: ProjectAccessRole,
    ) -> Result<Vec<i64>, AppError> {
        self.collaboration
            .project_ids_with_permission(project_ids, user_id, required)
            .await
    }
}
