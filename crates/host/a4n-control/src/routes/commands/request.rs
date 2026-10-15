use std::future::Future;

use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::ApiError;

pub(in crate::routes::commands) async fn run_with_runtime_context<F>(
    user_id: i64,
    snapshot: Option<a4n_runtime::host_api::runtime_config::RuntimeConfigSnapshot>,
    future: F,
) -> Result<Value, ApiError>
where
    F: Future<Output = Result<Value, ApiError>> + Send + 'static,
{
    // Dynamic Agent tools can enter this dispatcher from an Agent worker that
    // already owns a process-local snapshot. `tokio::spawn` does not inherit
    // task-local values, so capture the parent snapshot before detaching the
    // request task. An explicit request snapshot still wins.
    let snapshot = snapshot.or_else(a4n_runtime::host_api::runtime_config::current_snapshot);
    tokio::spawn(async move {
        a4n_runtime::host_api::runtime_config::request_scope(user_id, snapshot, future).await
    })
    .await
    .map_err(|error| {
        a4n_error::AppError::Internal(anyhow::anyhow!("Web command 后台任务异常退出: {error}"))
    })?
}

pub(in crate::routes::commands) async fn run_with_runtime_context_cancellable<F>(
    user_id: i64,
    snapshot: Option<a4n_runtime::host_api::runtime_config::RuntimeConfigSnapshot>,
    cancellation: CancellationToken,
    future: F,
) -> Result<Value, ApiError>
where
    F: Future<Output = Result<Value, ApiError>> + Send + 'static,
{
    let snapshot = snapshot.or_else(a4n_runtime::host_api::runtime_config::current_snapshot);
    let command = a4n_runtime::host_api::runtime_config::request_scope(user_id, snapshot, future);
    tokio::select! {
        biased;
        _ = cancellation.cancelled() => {
            Err(ApiError::page_operation_cancelled())
        }
        result = command => result,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;
    use tokio::sync::oneshot;

    use super::{run_with_runtime_context, run_with_runtime_context_cancellable};
    use tokio_util::sync::CancellationToken;

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

    #[tokio::test]
    async fn detached_command_inherits_parent_runtime_snapshot() {
        let snapshot = a4n_runtime::host_api::runtime_config::RuntimeConfigSnapshot {
            schema_version: 2,
            llm: None,
            llm_configs: Vec::new(),
            image: None,
            image_configs: Vec::new(),
            video: None,
            video_configs: Vec::new(),
            embedding: None,
            web_fetch: None,
            web_search: None,
            llm_default_source: None,
            image_default_source: None,
            video_default_source: None,
            system_llm_api_key_encrypted: None,
            system_image_api_key_encrypted: None,
            system_video_api_key_encrypted: None,
            client_context: None,
        };

        let result = a4n_runtime::host_api::runtime_config::scope(snapshot.clone(), async {
            run_with_runtime_context(1, None, async move {
                let inherited = a4n_runtime::host_api::runtime_config::current_snapshot();
                assert_eq!(inherited, Some(snapshot));
                Ok(json!({"inherited": true}))
            })
            .await
        })
        .await
        .expect("detached command should complete");
        assert_eq!(result["inherited"], json!(true));
    }

    #[tokio::test]
    async fn cancellable_command_returns_cancelled_and_drops_future() {
        let (started_tx, started_rx) = oneshot::channel();
        let cancellation = CancellationToken::new();
        let task = tokio::spawn(run_with_runtime_context_cancellable(
            1,
            None,
            cancellation.clone(),
            async move {
                let _ = started_tx.send(());
                std::future::pending::<()>().await;
                Ok(json!({"completed": true}))
            },
        ));

        started_rx.await.expect("cancellable command should start");
        cancellation.cancel();
        let error = task
            .await
            .expect("cancellable waiter should finish")
            .unwrap_err();
        assert_eq!(error.into_value()["code"], "page_operation_cancelled");
    }
}
