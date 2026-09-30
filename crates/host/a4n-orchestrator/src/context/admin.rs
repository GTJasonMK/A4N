use super::OrchestratorContext;
use crate::admin::{AdminPermission, require_admin_role};
use a4n_domain::auth::{UserRole, parse_user_role};
use a4n_error::AppError;

impl OrchestratorContext {
    pub(crate) async fn require_admin_permission(
        &self,
        caller_user_id: i64,
        permission: AdminPermission,
    ) -> Result<(), AppError> {
        let user = self.auth_user_authorization_by_id(caller_user_id).await?;
        if !user.is_active {
            return Err(AppError::Forbidden(format!("用户 {caller_user_id} 已停用")));
        }
        require_admin_role(&user.role, permission)
    }

    pub(crate) async fn is_active_admin_user(&self, user_id: i64) -> Result<bool, AppError> {
        let user = self.auth_user_authorization_by_id(user_id).await?;
        if !user.is_active {
            return Ok(false);
        }
        Ok(parse_user_role(&user.role)? == UserRole::Admin)
    }
}
