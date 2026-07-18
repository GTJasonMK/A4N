use crate::AppState;
use a4n_core::{
    error::AppError,
    models::user::role::{UserRole, parse_user_role},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdminPermission {
    ManageUsers,
    ManageSettings,
}

pub(crate) async fn require_admin_permission(
    state: &AppState,
    caller_user_id: i64,
    permission: AdminPermission,
) -> Result<(), AppError> {
    let user = state.orchestrator.auth_user_by_id(caller_user_id).await?;
    if !user.is_active {
        return Err(AppError::Forbidden(format!("用户 {caller_user_id} 已停用")));
    }
    require_admin_role(&user.role, permission)
}

pub(crate) async fn is_active_admin_user(state: &AppState, user_id: i64) -> Result<bool, AppError> {
    let user = state.orchestrator.auth_user_by_id(user_id).await?;
    if !user.is_active {
        return Ok(false);
    }
    Ok(parse_user_role(&user.role)? == UserRole::Admin)
}

pub(super) fn require_admin_role(role: &str, permission: AdminPermission) -> Result<(), AppError> {
    let role = parse_user_role(role)?;
    if role == UserRole::Admin {
        Ok(())
    } else {
        Err(AppError::Forbidden(format!(
            "角色 {role:?} 无权执行操作 {permission:?}"
        )))
    }
}
