use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use a4n_domain::collaboration::{
    ProjectAccessRole, ProjectAccessScope, ProjectInviteSummary, ProjectMemberRole,
    ProjectMemberSummary,
};

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InviteProjectMemberInput {
    pub username_or_email: String,
    pub role: ProjectMemberRole,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateProjectMemberRoleInput {
    pub role: ProjectMemberRole,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub struct ProjectCollaborationRootSummary {
    pub id: i64,
    pub user_id: i64,
}

/// Host-facing project access boundary.
///
/// Permission checks and collaboration-root resolution stay in the project
/// owner; Runtime consumes only roles and a small identity summary.
#[async_trait::async_trait]
pub trait ProjectAccessApplicationPort: Send + Sync {
    async fn verify_permission(
        &self,
        user_id: i64,
        project_id: i64,
        required: ProjectAccessRole,
    ) -> Result<ProjectAccessRole, a4n_error::AppError>;

    async fn verify_permissions(
        &self,
        user_id: i64,
        project_ids: &[i64],
        required: ProjectAccessRole,
    ) -> Result<(), a4n_error::AppError>;

    async fn project_ids_with_permission(
        &self,
        user_id: i64,
        project_ids: &[i64],
        required: ProjectAccessRole,
    ) -> Result<Vec<i64>, a4n_error::AppError>;

    async fn collaboration_root(
        &self,
        project_id: i64,
    ) -> Result<ProjectCollaborationRootSummary, a4n_error::AppError>;
}

/// Host-facing project collaboration boundary.
///
/// Membership, invitations and their state transitions stay owned by the
/// project collaboration application. Hosts receive stable DTOs and do not
/// depend on the orchestrator facade or persistence models.
#[async_trait::async_trait]
pub trait ProjectCollaborationApplicationPort: Send + Sync {
    async fn list_project_members(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ProjectMemberSummary>, a4n_error::AppError>;

    async fn invite_project_member(
        &self,
        user_id: i64,
        project_id: i64,
        input: InviteProjectMemberInput,
    ) -> Result<(), a4n_error::AppError>;

    async fn list_project_invites(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ProjectInviteSummary>, a4n_error::AppError>;

    async fn list_my_project_invites(
        &self,
        user_id: i64,
    ) -> Result<Vec<ProjectInviteSummary>, a4n_error::AppError>;

    async fn accept_project_invite(
        &self,
        user_id: i64,
        invite_id: i64,
    ) -> Result<(), a4n_error::AppError>;

    async fn decline_project_invite(
        &self,
        user_id: i64,
        invite_id: i64,
    ) -> Result<(), a4n_error::AppError>;

    async fn revoke_project_invite(
        &self,
        user_id: i64,
        project_id: i64,
        invite_id: i64,
    ) -> Result<(), a4n_error::AppError>;

    async fn update_project_member_role(
        &self,
        user_id: i64,
        project_id: i64,
        member_user_id: i64,
        input: UpdateProjectMemberRoleInput,
    ) -> Result<(), a4n_error::AppError>;

    async fn remove_project_member(
        &self,
        user_id: i64,
        project_id: i64,
        member_user_id: i64,
    ) -> Result<(), a4n_error::AppError>;

    async fn leave_project(&self, user_id: i64, project_id: i64)
    -> Result<(), a4n_error::AppError>;
}
