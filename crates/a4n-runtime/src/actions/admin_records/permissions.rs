use crate::AppState;
pub(crate) use a4n_api_contract::admin::AdminPermission;
use a4n_error::AppError;

pub(crate) async fn require_admin_permission(
    state: &AppState,
    caller_user_id: i64,
    permission: AdminPermission,
) -> Result<(), AppError> {
    state
        .admin_application()
        .require_permission(caller_user_id, permission)
        .await
}

pub(crate) async fn is_active_admin_user(state: &AppState, user_id: i64) -> Result<bool, AppError> {
    state
        .admin_application()
        .is_active_admin_user(user_id)
        .await
}
