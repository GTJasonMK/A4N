use a4n_core::error::AppError;

use super::errors::collaboration_db_error_to_app_error;
use super::{ProjectInviteSummary, ProjectMemberRole, ProjectMemberSummary};
use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn invite_project_member(
        &self,
        inviter_user_id: i64,
        project_id: i64,
        username_or_email: &str,
        role: ProjectMemberRole,
    ) -> Result<ProjectInviteSummary, AppError> {
        a4n_db::collaboration::invite_project_member(
            self.db.connection(),
            inviter_user_id,
            project_id,
            username_or_email,
            role,
        )
        .await
        .map_err(collaboration_db_error_to_app_error)
    }

    pub async fn list_project_invites(
        &self,
        actor_user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ProjectInviteSummary>, AppError> {
        a4n_db::collaboration::list_project_invites(self.db.connection(), actor_user_id, project_id)
            .await
            .map_err(collaboration_db_error_to_app_error)
    }

    pub async fn list_my_project_invites(
        &self,
        user_id: i64,
    ) -> Result<Vec<ProjectInviteSummary>, AppError> {
        a4n_db::collaboration::list_my_project_invites(self.db.connection(), user_id)
            .await
            .map_err(collaboration_db_error_to_app_error)
    }

    pub async fn accept_project_invite(
        &self,
        user_id: i64,
        invite_id: i64,
    ) -> Result<ProjectMemberSummary, AppError> {
        a4n_db::collaboration::accept_project_invite(self.db.connection(), user_id, invite_id)
            .await
            .map_err(collaboration_db_error_to_app_error)
    }

    pub async fn decline_project_invite(
        &self,
        user_id: i64,
        invite_id: i64,
    ) -> Result<ProjectInviteSummary, AppError> {
        a4n_db::collaboration::decline_project_invite(self.db.connection(), user_id, invite_id)
            .await
            .map_err(collaboration_db_error_to_app_error)
    }

    pub async fn revoke_project_invite(
        &self,
        actor_user_id: i64,
        project_id: i64,
        invite_id: i64,
    ) -> Result<ProjectInviteSummary, AppError> {
        a4n_db::collaboration::revoke_project_invite(
            self.db.connection(),
            actor_user_id,
            project_id,
            invite_id,
        )
        .await
        .map_err(collaboration_db_error_to_app_error)
    }
}
