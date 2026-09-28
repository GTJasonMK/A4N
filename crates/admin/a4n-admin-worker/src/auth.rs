use super::*;

impl WorkerState {
    pub(super) fn caller(
        &self,
        principal: &WorkerPrincipal,
        payload_user_id: i64,
    ) -> Result<i64, WorkerFailure> {
        if principal.kind != WorkerPrincipalKind::User {
            return Err(WorkerFailure::unauthorized(
                "admin worker 仅允许 user principal",
            ));
        }
        let caller = principal
            .subject
            .parse::<i64>()
            .map_err(|_| WorkerFailure::unauthorized("用户主体不是有效用户 ID"))?;
        if caller <= 0 || caller != payload_user_id {
            return Err(WorkerFailure::unauthorized("用户主体与请求调用者不匹配"));
        }
        Ok(caller)
    }

    pub(super) async fn require_admin(
        &self,
        caller_user_id: i64,
        _permission: AdminPermission,
    ) -> Result<(), WorkerFailure> {
        let user = a4n_db::auth::user_by_id(self.database.connection(), caller_user_id)
            .await
            .map_err(app_error)?;
        if !user.is_active {
            return Err(WorkerFailure::forbidden(format!(
                "用户 {caller_user_id} 已停用"
            )));
        }
        if parse_user_role(&user.role).map_err(app_error)?
            != a4n_db_models::models::user::role::UserRole::Admin
        {
            return Err(WorkerFailure::forbidden("当前用户不是管理员"));
        }
        Ok(())
    }

    pub(super) fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.as_bytes().ct_eq(self.auth_token.as_bytes()).into())
    }
}
