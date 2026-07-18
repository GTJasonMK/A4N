use a4n_core::error::AppError;

use super::errors::collaboration_db_error_to_app_error;
use super::{ProjectMemberRole, ProjectMemberSummary};
use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn list_project_members(
        &self,
        actor_user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ProjectMemberSummary>, AppError> {
        a4n_db::collaboration::list_project_members(self.db.connection(), actor_user_id, project_id)
            .await
            .map_err(collaboration_db_error_to_app_error)
    }

    pub async fn update_project_member_role(
        &self,
        actor_user_id: i64,
        project_id: i64,
        member_user_id: i64,
        role: ProjectMemberRole,
    ) -> Result<ProjectMemberSummary, AppError> {
        a4n_db::collaboration::update_project_member_role(
            self.db.connection(),
            actor_user_id,
            project_id,
            member_user_id,
            role,
        )
        .await
        .map_err(collaboration_db_error_to_app_error)
    }

    pub async fn remove_project_member(
        &self,
        actor_user_id: i64,
        project_id: i64,
        member_user_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::collaboration::remove_project_member(
            self.db.connection(),
            actor_user_id,
            project_id,
            member_user_id,
        )
        .await
        .map_err(collaboration_db_error_to_app_error)
    }

    pub async fn leave_project(&self, user_id: i64, project_id: i64) -> Result<(), AppError> {
        a4n_db::collaboration::leave_project(self.db.connection(), user_id, project_id)
            .await
            .map_err(collaboration_db_error_to_app_error)
    }
}
