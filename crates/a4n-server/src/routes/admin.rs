use super::auth::authenticated_user;
use super::error::ApiError;
use crate::state::ServerState;
use a4n_runtime::actions::admin_records;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;

pub async fn list_users(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<admin_records::AdminUserSummary>>, ApiError> {
    let caller = authenticated_user(&state, &headers).await?;
    Ok(Json(
        admin_records::list_all_users_for_admin(&state, caller.id).await?,
    ))
}

pub async fn create_user(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<admin_records::CreateUserInput>,
) -> Result<Json<admin_records::AdminUserSummary>, ApiError> {
    let caller = authenticated_user(&state, &headers).await?;
    Ok(Json(
        admin_records::create_user_for_admin(&state, caller.id, input).await?,
    ))
}

pub async fn list_invite_codes(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<admin_records::InviteCodeSummary>>, ApiError> {
    let caller = authenticated_user(&state, &headers).await?;
    Ok(Json(
        admin_records::list_invite_codes_for_admin(&state, caller.id).await?,
    ))
}

pub async fn create_invite_code(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<admin_records::CreateInviteCodeInput>,
) -> Result<Json<admin_records::InviteCodeSummary>, ApiError> {
    let caller = authenticated_user(&state, &headers).await?;
    Ok(Json(
        admin_records::create_invite_code_for_admin(&state, caller.id, input).await?,
    ))
}

pub async fn update_user_role(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(user_id): Path<i64>,
    Json(input): Json<admin_records::UpdateUserRoleInput>,
) -> Result<Json<admin_records::AdminUserSummary>, ApiError> {
    let caller = authenticated_user(&state, &headers).await?;
    Ok(Json(
        admin_records::update_user_role_for_admin(&state, caller.id, user_id, input).await?,
    ))
}

pub async fn disable_user(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(user_id): Path<i64>,
) -> Result<(), ApiError> {
    let caller = authenticated_user(&state, &headers).await?;
    admin_records::delete_user_for_admin(&state, caller.id, user_id).await?;
    Ok(())
}
