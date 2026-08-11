use crate::AppState;
use a4n_core::error::AppError;
pub(crate) use a4n_orchestrator::admin::AdminPermission;

pub(crate) async fn require_admin_permission(
    state: &AppState,
    caller_user_id: i64,
    permission: AdminPermission,
) -> Result<(), AppError> {
    state
        .orchestrator()
        .require_admin_permission(caller_user_id, permission)
        .await
}

pub(crate) async fn is_active_admin_user(state: &AppState, user_id: i64) -> Result<bool, AppError> {
    state.orchestrator().is_active_admin_user(user_id).await
}
