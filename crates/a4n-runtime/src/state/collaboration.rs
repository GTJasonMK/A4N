use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_orchestrator::{
    OrchestratorContext,
    collaboration::{
        AccessibleProjectListInput, AccessibleProjectsPage, CollaborationRootProject,
        ProjectAccessInfo, ProjectAccessInfoQuery, ProjectAccessRole, ProjectInviteSummary,
        ProjectMemberRole, ProjectMemberSummary,
    },
};

pub(crate) struct CollaborationRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> CollaborationRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn list_project_members(
        &self,
        actor_user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ProjectMemberSummary>, AppError> {
        self.orchestrator
            .list_project_members(actor_user_id, project_id)
            .await
    }

    pub(crate) async fn invite_project_member(
        &self,
        inviter_user_id: i64,
        project_id: i64,
        username_or_email: &str,
        role: ProjectMemberRole,
    ) -> Result<ProjectInviteSummary, AppError> {
        self.orchestrator
            .invite_project_member(inviter_user_id, project_id, username_or_email, role)
            .await
    }

    pub(crate) async fn list_project_invites(
        &self,
        actor_user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ProjectInviteSummary>, AppError> {
        self.orchestrator
            .list_project_invites(actor_user_id, project_id)
            .await
    }

    pub(crate) async fn list_my_project_invites(
        &self,
        user_id: i64,
    ) -> Result<Vec<ProjectInviteSummary>, AppError> {
        self.orchestrator.list_my_project_invites(user_id).await
    }

    pub(crate) async fn accept_project_invite(
        &self,
        user_id: i64,
        invite_id: i64,
    ) -> Result<ProjectMemberSummary, AppError> {
        self.orchestrator
            .accept_project_invite(user_id, invite_id)
            .await
    }

    pub(crate) async fn decline_project_invite(
        &self,
        user_id: i64,
        invite_id: i64,
    ) -> Result<ProjectInviteSummary, AppError> {
        self.orchestrator
            .decline_project_invite(user_id, invite_id)
            .await
    }

    pub(crate) async fn revoke_project_invite(
        &self,
        actor_user_id: i64,
        project_id: i64,
        invite_id: i64,
    ) -> Result<ProjectInviteSummary, AppError> {
        self.orchestrator
            .revoke_project_invite(actor_user_id, project_id, invite_id)
            .await
    }

    pub(crate) async fn update_project_member_role(
        &self,
        actor_user_id: i64,
        project_id: i64,
        member_user_id: i64,
        role: ProjectMemberRole,
    ) -> Result<ProjectMemberSummary, AppError> {
        self.orchestrator
            .update_project_member_role(actor_user_id, project_id, member_user_id, role)
            .await
    }

    pub(crate) async fn remove_project_member(
        &self,
        actor_user_id: i64,
        project_id: i64,
        member_user_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .remove_project_member(actor_user_id, project_id, member_user_id)
            .await
    }

    pub(crate) async fn leave_project(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator.leave_project(user_id, project_id).await
    }

    pub(crate) async fn collaboration_root_project(
        &self,
        project_id: i64,
    ) -> Result<CollaborationRootProject, AppError> {
        self.orchestrator
            .collaboration_root_project(project_id)
            .await
    }

    pub(crate) async fn accessible_projects(
        &self,
        input: AccessibleProjectListInput,
    ) -> Result<AccessibleProjectsPage, AppError> {
        self.orchestrator.accessible_projects(input).await
    }

    pub(crate) async fn project_access_info(
        &self,
        query: ProjectAccessInfoQuery,
        user_id: i64,
    ) -> Result<ProjectAccessInfo, AppError> {
        self.orchestrator.project_access_info(query, user_id).await
    }

    pub(crate) async fn project_access_info_batch(
        &self,
        queries: &[ProjectAccessInfoQuery],
        user_id: i64,
    ) -> Result<HashMap<i64, ProjectAccessInfo>, AppError> {
        self.orchestrator
            .project_access_info_batch(queries, user_id)
            .await
    }

    pub(crate) async fn verify_project_permission(
        &self,
        project_id: i64,
        user_id: i64,
        required: ProjectAccessRole,
    ) -> Result<ProjectAccessRole, AppError> {
        self.orchestrator
            .verify_project_permission(project_id, user_id, required)
            .await
    }

    pub(crate) async fn verify_project_permissions(
        &self,
        project_ids: &[i64],
        user_id: i64,
        required: ProjectAccessRole,
    ) -> Result<(), AppError> {
        self.orchestrator
            .verify_project_permissions(project_ids, user_id, required)
            .await
    }
}
