use super::{
    AdminUserSummary, CreateInviteCodeInput, CreateUserInput, InviteCodeSummary,
    UpdateUserRoleInput,
};
use crate::{
    AppState,
    actions::{app_error_message, auth_records::to_user_summary},
};
use a4n_core::models::user::{invite_code, user};

pub async fn list_all_users(state: &AppState) -> Result<Vec<AdminUserSummary>, String> {
    state
        .orchestrator()
        .all_user_summaries()
        .await
        .map_err(app_error_message)
}

pub async fn create_user(
    state: &AppState,
    input: CreateUserInput,
) -> Result<AdminUserSummary, String> {
    let user = state
        .orchestrator()
        .create_auth_user(&input.username, &input.email, &input.password, &input.role)
        .await
        .map_err(|error| error.to_string())?;
    Ok(admin_user_summary(&user))
}

pub async fn list_invite_codes(state: &AppState) -> Result<Vec<InviteCodeSummary>, String> {
    state
        .orchestrator()
        .invite_code_summaries()
        .await
        .map_err(app_error_message)
}

pub async fn create_invite_code(
    state: &AppState,
    caller_user_id: i64,
    input: CreateInviteCodeInput,
) -> Result<InviteCodeSummary, String> {
    let code = state
        .orchestrator()
        .create_invite_code_record(&input.code, Some(caller_user_id))
        .await
        .map_err(|error| error.to_string())?;
    Ok(invite_code_summary(&code))
}

pub async fn update_user_role(
    state: &AppState,
    user_id: i64,
    input: UpdateUserRoleInput,
) -> Result<AdminUserSummary, String> {
    let user = state
        .orchestrator()
        .update_auth_user_role(user_id, &input.role)
        .await
        .map_err(|error| error.to_string())?;
    Ok(admin_user_summary(&user))
}

pub async fn disable_user(
    state: &AppState,
    caller_user_id: i64,
    user_id: i64,
) -> Result<(), String> {
    state
        .destructive_lifecycle()
        .disable_user(state, caller_user_id, user_id)
        .await
}

fn admin_user_summary(user: &user::Model) -> AdminUserSummary {
    let base = to_user_summary(user).with_display_name();
    AdminUserSummary {
        id: base.id,
        username: base.username,
        email: base.email,
        role: base.role,
        oauth_provider: base.oauth_provider,
        oauth_subject: base.oauth_subject,
        avatar_url: base.avatar_url,
        is_active: base.is_active,
        display_name: base.display_name,
        created_at: base.created_at,
        updated_at: base.updated_at,
        last_login_at: base.last_login_at,
    }
}

fn invite_code_summary(code: &invite_code::Model) -> InviteCodeSummary {
    InviteCodeSummary {
        id: code.id,
        code: code.code.clone(),
        created_by_user_id: code.created_by_user_id,
        used_by_user_id: code.used_by_user_id,
        created_at: Some(code.created_at.to_string()),
        used_at: code.used_at.map(|value| value.to_string()),
    }
}
