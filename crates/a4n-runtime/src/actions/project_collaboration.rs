use crate::AppState;
use crate::actions::app_error_message;
pub use a4n_orchestrator::collaboration::{ProjectInviteSummary, ProjectMemberSummary};
use a4n_orchestrator::collaboration::{ProjectMemberRole, parse_project_member_role};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InviteProjectMemberInput {
    pub username_or_email: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateProjectMemberRoleInput {
    pub role: String,
}

pub async fn list_project_members(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ProjectMemberSummary>, String> {
    state
        .collaboration()
        .list_project_members(user_id, project_id)
        .await
        .map_err(app_error_message)
}

pub async fn invite_project_member(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: InviteProjectMemberInput,
) -> Result<ProjectInviteSummary, String> {
    let role = parse_role(&input.role)?;
    state
        .collaboration()
        .invite_project_member(user_id, project_id, &input.username_or_email, role)
        .await
        .map_err(app_error_message)
}

pub async fn list_project_invites(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ProjectInviteSummary>, String> {
    state
        .collaboration()
        .list_project_invites(user_id, project_id)
        .await
        .map_err(app_error_message)
}

pub async fn list_my_project_invites(
    state: &AppState,
    user_id: i64,
) -> Result<Vec<ProjectInviteSummary>, String> {
    state
        .collaboration()
        .list_my_project_invites(user_id)
        .await
        .map_err(app_error_message)
}

pub async fn accept_project_invite(
    state: &AppState,
    user_id: i64,
    invite_id: i64,
) -> Result<ProjectMemberSummary, String> {
    state
        .collaboration()
        .accept_project_invite(user_id, invite_id)
        .await
        .map_err(app_error_message)
}

pub async fn decline_project_invite(
    state: &AppState,
    user_id: i64,
    invite_id: i64,
) -> Result<ProjectInviteSummary, String> {
    state
        .collaboration()
        .decline_project_invite(user_id, invite_id)
        .await
        .map_err(app_error_message)
}

pub async fn revoke_project_invite(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    invite_id: i64,
) -> Result<ProjectInviteSummary, String> {
    state
        .collaboration()
        .revoke_project_invite(user_id, project_id, invite_id)
        .await
        .map_err(app_error_message)
}

pub async fn update_project_member_role(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    member_user_id: i64,
    input: UpdateProjectMemberRoleInput,
) -> Result<ProjectMemberSummary, String> {
    let role = parse_role(&input.role)?;
    state
        .collaboration()
        .update_project_member_role(user_id, project_id, member_user_id, role)
        .await
        .map_err(app_error_message)
}

pub async fn remove_project_member(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    member_user_id: i64,
) -> Result<(), String> {
    state
        .collaboration()
        .remove_project_member(user_id, project_id, member_user_id)
        .await
        .map_err(app_error_message)
}

pub async fn leave_project(state: &AppState, user_id: i64, project_id: i64) -> Result<(), String> {
    state
        .collaboration()
        .leave_project(user_id, project_id)
        .await
        .map_err(app_error_message)
}

fn parse_role(value: &str) -> Result<ProjectMemberRole, String> {
    parse_project_member_role(value).map_err(|error| error.to_string())
}
