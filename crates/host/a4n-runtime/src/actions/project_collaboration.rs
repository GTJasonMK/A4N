use crate::AppState;
use crate::actions::app_error_message;
pub use a4n_api_contract::collaboration::{
    InviteProjectMemberInput, ProjectInviteSummary, ProjectMemberSummary,
    UpdateProjectMemberRoleInput,
};

pub async fn list_project_members(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ProjectMemberSummary>, String> {
    state
        .project_collaboration_application()
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
    state
        .project_collaboration_application()
        .invite_project_member(user_id, project_id, input)
        .await
        .map_err(app_error_message)
}

pub async fn list_project_invites(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ProjectInviteSummary>, String> {
    state
        .project_collaboration_application()
        .list_project_invites(user_id, project_id)
        .await
        .map_err(app_error_message)
}

pub async fn list_my_project_invites(
    state: &AppState,
    user_id: i64,
) -> Result<Vec<ProjectInviteSummary>, String> {
    state
        .project_collaboration_application()
        .list_my_project_invites(user_id)
        .await
        .map_err(app_error_message)
}

pub async fn accept_project_invite(
    state: &AppState,
    user_id: i64,
    invite_id: i64,
) -> Result<(), String> {
    state
        .project_collaboration_application()
        .accept_project_invite(user_id, invite_id)
        .await
        .map_err(app_error_message)
}

pub async fn decline_project_invite(
    state: &AppState,
    user_id: i64,
    invite_id: i64,
) -> Result<(), String> {
    state
        .project_collaboration_application()
        .decline_project_invite(user_id, invite_id)
        .await
        .map_err(app_error_message)
}

pub async fn revoke_project_invite(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    invite_id: i64,
) -> Result<(), String> {
    state
        .project_collaboration_application()
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
) -> Result<(), String> {
    state
        .project_collaboration_application()
        .update_project_member_role(user_id, project_id, member_user_id, input)
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
        .project_collaboration_application()
        .remove_project_member(user_id, project_id, member_user_id)
        .await
        .map_err(app_error_message)
}

pub async fn leave_project(state: &AppState, user_id: i64, project_id: i64) -> Result<(), String> {
    state
        .project_collaboration_application()
        .leave_project(user_id, project_id)
        .await
        .map_err(app_error_message)
}
