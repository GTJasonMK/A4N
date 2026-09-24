use super::{
    CreateInviteCodeInput, CreateUserInput, InviteCodeSummary, UpdateUserRoleInput, UserSummary,
};
use crate::AppState;

pub(super) async fn list_all_users(
    state: &AppState,
    caller_user_id: i64,
) -> Result<Vec<UserSummary>, String> {
    let users = state
        .admin_read_application()
        .list_users(caller_user_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(users)
}

pub(super) async fn create_user(
    state: &AppState,
    caller_user_id: i64,
    input: CreateUserInput,
) -> Result<UserSummary, String> {
    state
        .admin_mutation_application()
        .create_user(caller_user_id, input)
        .await
        .map_err(|error| error.to_string())
}

pub(super) async fn list_invite_codes(
    state: &AppState,
    caller_user_id: i64,
) -> Result<Vec<InviteCodeSummary>, String> {
    state
        .admin_read_application()
        .list_invite_codes(caller_user_id)
        .await
        .map_err(|error| error.to_string())
}

pub(super) async fn create_invite_code(
    state: &AppState,
    caller_user_id: i64,
    input: CreateInviteCodeInput,
) -> Result<InviteCodeSummary, String> {
    state
        .admin_mutation_application()
        .create_invite_code(caller_user_id, input)
        .await
        .map_err(|error| error.to_string())
}

pub(super) async fn update_user_role(
    state: &AppState,
    caller_user_id: i64,
    user_id: i64,
    input: UpdateUserRoleInput,
) -> Result<UserSummary, String> {
    state
        .admin_mutation_application()
        .update_user_role(caller_user_id, user_id, input)
        .await
        .map_err(|error| error.to_string())
}

pub(super) async fn disable_user(
    state: &AppState,
    caller_user_id: i64,
    user_id: i64,
) -> Result<(), String> {
    state
        .destructive_lifecycle()
        .disable_user(state, caller_user_id, user_id)
        .await
}
