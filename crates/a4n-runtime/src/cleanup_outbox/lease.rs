use crate::AppState;
use crate::actions::app_error_message;
use std::future::Future;
use std::time::Duration;

const LEASE_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);

pub(super) async fn renew_cleanup_lease(
    state: &AppState,
    id: i64,
    lease_token: &str,
) -> Result<(), String> {
    state
        .orchestrator
        .renew_claimed_cleanup(id, lease_token)
        .await
        .map_err(app_error_message)
}

pub(super) async fn run_blocking_with_lease_heartbeat<T, F>(
    state: &AppState,
    id: i64,
    lease_token: &str,
    operation: F,
) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    renew_cleanup_lease(state, id, lease_token).await?;
    let mut task = tokio::task::spawn_blocking(operation);
    loop {
        tokio::select! {
            result = &mut task => {
                return result.map_err(|error| format!("项目 cleanup 文件任务执行失败: {error}"))?;
            }
            _ = tokio::time::sleep(LEASE_HEARTBEAT_INTERVAL) => {
                if let Err(error) = renew_cleanup_lease(state, id, lease_token).await {
                    return match task.await {
                        Ok(Ok(_)) => Err(error),
                        Ok(Err(operation_error)) => Err(format!(
                            "{error}；项目 cleanup 文件任务同时失败: {operation_error}"
                        )),
                        Err(join_error) => Err(format!(
                            "{error}；项目 cleanup 文件任务同时异常退出: {join_error}"
                        )),
                    };
                }
            }
        }
    }
}

pub(super) async fn run_with_lease_heartbeat<T, E, F>(
    state: &AppState,
    id: i64,
    lease_token: &str,
    operation: F,
) -> Result<T, String>
where
    E: std::fmt::Display,
    F: Future<Output = Result<T, E>>,
{
    tokio::pin!(operation);
    loop {
        tokio::select! {
            result = &mut operation => return result.map_err(|error| error.to_string()),
            _ = tokio::time::sleep(LEASE_HEARTBEAT_INTERVAL) => {
                renew_cleanup_lease(state, id, lease_token).await?;
            }
        }
    }
}

pub(super) fn cleanup_retry_after_seconds(attempt_count: i32) -> i64 {
    let exponent = u32::try_from((i64::from(attempt_count) - 1).clamp(0, 6))
        .expect("cleanup retry exponent is clamped to 0..=6");
    (5_i64 * (1_i64 << exponent)).min(300)
}
