use std::collections::HashMap;

use a4n_core::error::AppError;

use super::errors::collaboration_db_error_to_app_error;
use super::{
    AccessibleProjectListInput, AccessibleProjectsPage, CollaborationRootProject,
    ProjectAccessInfo, ProjectAccessInfoQuery, ProjectAccessRole,
};
use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn accessible_projects(
        &self,
        input: AccessibleProjectListInput,
    ) -> Result<AccessibleProjectsPage, AppError> {
        a4n_db::collaboration::list_accessible_projects(self.db.connection(), input)
            .await
            .map_err(collaboration_db_error_to_app_error)
    }

    pub async fn project_access_info(
        &self,
        query: ProjectAccessInfoQuery,
        user_id: i64,
    ) -> Result<ProjectAccessInfo, AppError> {
        a4n_db::collaboration::project_access_info(self.db.connection(), query, user_id)
            .await
            .map_err(collaboration_db_error_to_app_error)
    }

    pub async fn project_access_info_batch(
        &self,
        queries: &[ProjectAccessInfoQuery],
        user_id: i64,
    ) -> Result<HashMap<i64, ProjectAccessInfo>, AppError> {
        a4n_db::collaboration::project_access_info_batch(self.db.connection(), queries, user_id)
            .await
            .map_err(collaboration_db_error_to_app_error)
    }

    pub async fn collaboration_root_project(
        &self,
        project_id: i64,
    ) -> Result<CollaborationRootProject, AppError> {
        a4n_db::collaboration::resolve_root_project(self.db.connection(), project_id)
            .await
            .map_err(collaboration_db_error_to_app_error)
    }

    pub async fn verify_project_permission(
        &self,
        project_id: i64,
        user_id: i64,
        required: ProjectAccessRole,
    ) -> Result<ProjectAccessRole, AppError> {
        a4n_db::collaboration::verify_project_permission(
            self.db.connection(),
            project_id,
            user_id,
            required,
        )
        .await
        .map_err(collaboration_db_error_to_app_error)
    }

    pub async fn verify_project_permissions(
        &self,
        project_ids: &[i64],
        user_id: i64,
        required: ProjectAccessRole,
    ) -> Result<(), AppError> {
        a4n_db::collaboration::verify_project_permissions(
            self.db.connection(),
            project_ids,
            user_id,
            required,
        )
        .await
        .map_err(collaboration_db_error_to_app_error)
    }
}
