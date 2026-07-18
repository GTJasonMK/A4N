use std::future::Future;

use serde_json::Value;

use super::{ApiError, ServerState, i64_arg};

pub(in crate::routes::commands) async fn run_with_runtime_context<F>(
    user_id: i64,
    snapshot: Option<a4n_runtime::runtime_config::RuntimeConfigSnapshot>,
    future: F,
) -> Result<Value, ApiError>
where
    F: Future<Output = Result<Value, ApiError>>,
{
    a4n_runtime::runtime_config::request_scope(user_id, snapshot, future).await
}

pub(in crate::routes::commands) async fn require_admin_caller(
    _state: &ServerState,
    authenticated_user_id: i64,
    args: &Value,
) -> Result<(), ApiError> {
    let caller_user_id = i64_arg(args, "callerUserId")?;
    if caller_user_id != authenticated_user_id {
        return Err(ApiError::forbidden("请求用户与登录用户不一致"));
    }
    Ok(())
}
