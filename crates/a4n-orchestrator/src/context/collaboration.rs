use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_db::DbHandle;

use super::OrchestratorContext;
use crate::collaboration::{
    AccessibleProjectListInput, AccessibleProjectsPage, CollaborationRootProject,
    ProjectAccessInfo, ProjectAccessInfoQuery, ProjectAccessRole, ProjectInviteSummary,
    ProjectMemberRole, ProjectMemberSummary, collaboration_db_error_to_app_error,
};

#[derive(Clone)]
pub(super) struct CollaborationApplication {
    database: DbHandle,
}

impl CollaborationApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn accessible_projects(
        &self,
        input: AccessibleProjectListInput,
    ) -> Result<AccessibleProjectsPage, AppError> {
        a4n_db::collaboration::list_accessible_projects(self.database.connection(), input)
            .await
            .map_err(collaboration_db_error_to_app_error)
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

    async fn project_access_info_batch(
        &self,
        queries: &[ProjectAccessInfoQuery],
        user_id: i64,
    ) -> Result<HashMap<i64, ProjectAccessInfo>, AppError> {
        a4n_db::collaboration::project_access_info_batch(
            self.database.connection(),
            queries,
            user_id,
        )
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

    async fn invite_project_member(
        &self,
        inviter_user_id: i64,
        project_id: i64,
        username_or_email: &str,
        role: ProjectMemberRole,
    ) -> Result<ProjectInviteSummary, AppError> {
        a4n_db::collaboration::invite_project_member(
            self.database.connection(),
            inviter_user_id,
            project_id,
            username_or_email,
            role,
        )
        .await
        .map_err(collaboration_db_error_to_app_error)
    }

    async fn list_project_invites(
        &self,
        actor_user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ProjectInviteSummary>, AppError> {
        a4n_db::collaboration::list_project_invites(
            self.database.connection(),
            actor_user_id,
            project_id,
        )
        .await
        .map_err(collaboration_db_error_to_app_error)
    }

    async fn list_my_project_invites(
        &self,
        user_id: i64,
    ) -> Result<Vec<ProjectInviteSummary>, AppError> {
        a4n_db::collaboration::list_my_project_invites(self.database.connection(), user_id)
            .await
            .map_err(collaboration_db_error_to_app_error)
    }

    async fn accept_project_invite(
        &self,
        user_id: i64,
        invite_id: i64,
    ) -> Result<ProjectMemberSummary, AppError> {
        a4n_db::collaboration::accept_project_invite(self.database.connection(), user_id, invite_id)
            .await
            .map_err(collaboration_db_error_to_app_error)
    }

    async fn decline_project_invite(
        &self,
        user_id: i64,
        invite_id: i64,
    ) -> Result<ProjectInviteSummary, AppError> {
        a4n_db::collaboration::decline_project_invite(
            self.database.connection(),
            user_id,
            invite_id,
        )
        .await
        .map_err(collaboration_db_error_to_app_error)
    }

    async fn revoke_project_invite(
        &self,
        actor_user_id: i64,
        project_id: i64,
        invite_id: i64,
    ) -> Result<ProjectInviteSummary, AppError> {
        a4n_db::collaboration::revoke_project_invite(
            self.database.connection(),
            actor_user_id,
            project_id,
            invite_id,
        )
        .await
        .map_err(collaboration_db_error_to_app_error)
    }

    async fn list_project_members(
        &self,
        actor_user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ProjectMemberSummary>, AppError> {
        a4n_db::collaboration::list_project_members(
            self.database.connection(),
            actor_user_id,
            project_id,
        )
        .await
        .map_err(collaboration_db_error_to_app_error)
    }

    async fn update_project_member_role(
        &self,
        actor_user_id: i64,
        project_id: i64,
        member_user_id: i64,
        role: ProjectMemberRole,
    ) -> Result<ProjectMemberSummary, AppError> {
        a4n_db::collaboration::update_project_member_role(
            self.database.connection(),
            actor_user_id,
            project_id,
            member_user_id,
            role,
        )
        .await
        .map_err(collaboration_db_error_to_app_error)
    }

    async fn remove_project_member(
        &self,
        actor_user_id: i64,
        project_id: i64,
        member_user_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::collaboration::remove_project_member(
            self.database.connection(),
            actor_user_id,
            project_id,
            member_user_id,
        )
        .await
        .map_err(collaboration_db_error_to_app_error)
    }

    async fn leave_project(&self, user_id: i64, project_id: i64) -> Result<(), AppError> {
        a4n_db::collaboration::leave_project(self.database.connection(), user_id, project_id)
            .await
            .map_err(collaboration_db_error_to_app_error)
    }
}

impl OrchestratorContext {
    pub async fn accessible_projects(
        &self,
        input: AccessibleProjectListInput,
    ) -> Result<AccessibleProjectsPage, AppError> {
        self.collaboration.accessible_projects(input).await
    }

    pub async fn project_access_info(
        &self,
        query: ProjectAccessInfoQuery,
        user_id: i64,
    ) -> Result<ProjectAccessInfo, AppError> {
        self.collaboration.project_access_info(query, user_id).await
    }

    pub async fn project_access_info_batch(
        &self,
        queries: &[ProjectAccessInfoQuery],
        user_id: i64,
    ) -> Result<HashMap<i64, ProjectAccessInfo>, AppError> {
        self.collaboration
            .project_access_info_batch(queries, user_id)
            .await
    }

    pub async fn collaboration_root_project(
        &self,
        project_id: i64,
    ) -> Result<CollaborationRootProject, AppError> {
        self.collaboration
            .collaboration_root_project(project_id)
            .await
    }

    pub async fn verify_project_permission(
        &self,
        project_id: i64,
        user_id: i64,
        required: ProjectAccessRole,
    ) -> Result<ProjectAccessRole, AppError> {
        self.collaboration
            .verify_project_permission(project_id, user_id, required)
            .await
    }

    pub async fn verify_project_permissions(
        &self,
        project_ids: &[i64],
        user_id: i64,
        required: ProjectAccessRole,
    ) -> Result<(), AppError> {
        self.collaboration
            .verify_project_permissions(project_ids, user_id, required)
            .await
    }

    pub async fn invite_project_member(
        &self,
        inviter_user_id: i64,
        project_id: i64,
        username_or_email: &str,
        role: ProjectMemberRole,
    ) -> Result<ProjectInviteSummary, AppError> {
        self.collaboration
            .invite_project_member(inviter_user_id, project_id, username_or_email, role)
            .await
    }

    pub async fn list_project_invites(
        &self,
        actor_user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ProjectInviteSummary>, AppError> {
        self.collaboration
            .list_project_invites(actor_user_id, project_id)
            .await
    }

    pub async fn list_my_project_invites(
        &self,
        user_id: i64,
    ) -> Result<Vec<ProjectInviteSummary>, AppError> {
        self.collaboration.list_my_project_invites(user_id).await
    }

    pub async fn accept_project_invite(
        &self,
        user_id: i64,
        invite_id: i64,
    ) -> Result<ProjectMemberSummary, AppError> {
        self.collaboration
            .accept_project_invite(user_id, invite_id)
            .await
    }

    pub async fn decline_project_invite(
        &self,
        user_id: i64,
        invite_id: i64,
    ) -> Result<ProjectInviteSummary, AppError> {
        self.collaboration
            .decline_project_invite(user_id, invite_id)
            .await
    }

    pub async fn revoke_project_invite(
        &self,
        actor_user_id: i64,
        project_id: i64,
        invite_id: i64,
    ) -> Result<ProjectInviteSummary, AppError> {
        self.collaboration
            .revoke_project_invite(actor_user_id, project_id, invite_id)
            .await
    }

    pub async fn list_project_members(
        &self,
        actor_user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ProjectMemberSummary>, AppError> {
        self.collaboration
            .list_project_members(actor_user_id, project_id)
            .await
    }

    pub async fn update_project_member_role(
        &self,
        actor_user_id: i64,
        project_id: i64,
        member_user_id: i64,
        role: ProjectMemberRole,
    ) -> Result<ProjectMemberSummary, AppError> {
        self.collaboration
            .update_project_member_role(actor_user_id, project_id, member_user_id, role)
            .await
    }

    pub async fn remove_project_member(
        &self,
        actor_user_id: i64,
        project_id: i64,
        member_user_id: i64,
    ) -> Result<(), AppError> {
        self.collaboration
            .remove_project_member(actor_user_id, project_id, member_user_id)
            .await
    }

    pub async fn leave_project(&self, user_id: i64, project_id: i64) -> Result<(), AppError> {
        self.collaboration.leave_project(user_id, project_id).await
    }
}
