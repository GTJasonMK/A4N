use super::{
    CreateInviteCodeInput, CreateUserInput, InviteCodeSummary, UpdateUserRoleInput, UserSummary,
};
use crate::{
    AppState,
    actions::{app_error_message, auth_records::to_user_summary},
};
use a4n_core::models::user::invite_code;

pub async fn list_all_users(state: &AppState) -> Result<Vec<UserSummary>, String> {
    let users = state
        .orchestrator()
        .list_auth_users()
        .await
        .map_err(app_error_message)?;
    Ok(users.iter().map(to_user_summary).collect())
}

pub async fn create_user(state: &AppState, input: CreateUserInput) -> Result<UserSummary, String> {
    let user = state
        .orchestrator()
        .create_auth_user(&input.username, &input.email, &input.password, &input.role)
        .await
        .map_err(|error| error.to_string())?;
    Ok(to_user_summary(&user))
}

pub async fn list_invite_codes(state: &AppState) -> Result<Vec<InviteCodeSummary>, String> {
    let codes = state
        .orchestrator()
        .list_invite_code_records()
        .await
        .map_err(app_error_message)?;
    Ok(codes.iter().map(invite_code_summary).collect())
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
) -> Result<UserSummary, String> {
    let user = state
        .orchestrator()
        .update_auth_user_role(user_id, &input.role)
        .await
        .map_err(|error| error.to_string())?;
    Ok(to_user_summary(&user))
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
