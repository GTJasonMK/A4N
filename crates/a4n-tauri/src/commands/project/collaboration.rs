use a4n_runtime::AppState;
use a4n_runtime::actions::project_collaboration as collaboration_actions;
use a4n_runtime::actions::project_collaboration::{
    InviteProjectMemberInput, UpdateProjectMemberRoleInput,
};
use a4n_services::collaboration::{ProjectInviteSummary, ProjectMemberSummary};
use tauri::State;

#[tauri::command]
pub async fn list_project_members(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ProjectMemberSummary>, String> {
    collaboration_actions::list_project_members(&state, user_id, project_id).await
}

#[tauri::command]
pub async fn invite_project_member(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    input: InviteProjectMemberInput,
) -> Result<ProjectInviteSummary, String> {
    collaboration_actions::invite_project_member(&state, user_id, project_id, input).await
}

#[tauri::command]
pub async fn list_project_invites(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ProjectInviteSummary>, String> {
    collaboration_actions::list_project_invites(&state, user_id, project_id).await
}

#[tauri::command]
pub async fn list_my_project_invites(
    state: State<'_, AppState>,
    user_id: i64,
) -> Result<Vec<ProjectInviteSummary>, String> {
    collaboration_actions::list_my_project_invites(&state, user_id).await
}

#[tauri::command]
pub async fn accept_project_invite(
    state: State<'_, AppState>,
    user_id: i64,
    invite_id: i64,
) -> Result<ProjectMemberSummary, String> {
    collaboration_actions::accept_project_invite(&state, user_id, invite_id).await
}

#[tauri::command]
pub async fn decline_project_invite(
    state: State<'_, AppState>,
    user_id: i64,
    invite_id: i64,
) -> Result<ProjectInviteSummary, String> {
    collaboration_actions::decline_project_invite(&state, user_id, invite_id).await
}

#[tauri::command]
pub async fn revoke_project_invite(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    invite_id: i64,
) -> Result<ProjectInviteSummary, String> {
    collaboration_actions::revoke_project_invite(&state, user_id, project_id, invite_id).await
}

#[tauri::command]
pub async fn update_project_member_role(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    member_user_id: i64,
    input: UpdateProjectMemberRoleInput,
) -> Result<ProjectMemberSummary, String> {
    collaboration_actions::update_project_member_role(
        &state,
        user_id,
        project_id,
        member_user_id,
        input,
    )
    .await
}

#[tauri::command]
pub async fn remove_project_member(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    member_user_id: i64,
) -> Result<(), String> {
    collaboration_actions::remove_project_member(&state, user_id, project_id, member_user_id).await
}

#[tauri::command]
pub async fn leave_project(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<(), String> {
    collaboration_actions::leave_project(&state, user_id, project_id).await
}
