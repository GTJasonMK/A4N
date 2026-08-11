use std::future::Future;

use serde_json::Value;

use super::{ApiError, i64_arg};

pub(in crate::routes::commands) async fn run_with_runtime_context<F>(
    user_id: i64,
    snapshot: Option<a4n_runtime::runtime_config::RuntimeConfigSnapshot>,
    future: F,
) -> Result<Value, ApiError>
where
    F: Future<Output = Result<Value, ApiError>> + Send + 'static,
{
    tokio::spawn(async move {
        a4n_runtime::runtime_config::request_scope(user_id, snapshot, future).await
    })
    .await
    .map_err(|error| {
        a4n_core::error::AppError::Internal(anyhow::anyhow!(
            "Web command 后台任务异常退出: {error}"
        ))
    })?
}

pub(in crate::routes::commands) fn require_admin_caller(
    authenticated_user_id: i64,
    args: &Value,
) -> Result<(), ApiError> {
    let caller_user_id = i64_arg(args, "callerUserId")?;
    if caller_user_id != authenticated_user_id {
        return Err(ApiError::forbidden("请求用户与登录用户不一致"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;
    use tokio::sync::oneshot;

    use super::run_with_runtime_context;

    #[tokio::test]
    async fn detached_command_survives_dropped_http_waiter() {
        let (started_tx, started_rx) = oneshot::channel();
        let (release_tx, release_rx) = oneshot::channel();
        let (completed_tx, completed_rx) = oneshot::channel();
        let waiter = tokio::spawn(run_with_runtime_context(1, None, async move {
            let _ = started_tx.send(());
            let _ = release_rx.await;
            let _ = completed_tx.send(());
            Ok(json!({"completed": true}))
        }));

        started_rx.await.expect("command should start");
        waiter.abort();
        let _ = waiter.await;
        release_tx.send(()).expect("command should still be alive");
        tokio::time::timeout(Duration::from_secs(1), completed_rx)
            .await
            .expect("detached command should complete after waiter is dropped")
            .expect("completion signal should be sent");
    }
}
