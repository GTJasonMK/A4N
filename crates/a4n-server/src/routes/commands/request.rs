use std::future::Future;

use serde_json::Value;

use super::ApiError;

pub(in crate::routes::commands) async fn run_with_runtime_context<F>(
    user_id: i64,
    snapshot: Option<a4n_runtime::runtime_config::RuntimeConfigSnapshot>,
    future: F,
) -> Result<Value, ApiError>
where
    F: Future<Output = Result<Value, ApiError>> + Send + 'static,
{
    // Dynamic Agent tools can enter this dispatcher from an Agent worker that
    // already owns a process-local snapshot. `tokio::spawn` does not inherit
    // task-local values, so capture the parent snapshot before detaching the
    // request task. An explicit request snapshot still wins.
    let snapshot = snapshot.or_else(a4n_runtime::runtime_config::current_snapshot);
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

    #[tokio::test]
    async fn detached_command_inherits_parent_runtime_snapshot() {
        let snapshot = a4n_runtime::runtime_config::RuntimeConfigSnapshot {
            schema_version: 1,
            llm: None,
            llm_configs: Vec::new(),
            image: None,
            image_configs: Vec::new(),
            video: None,
            video_configs: Vec::new(),
            embedding: None,
            llm_default_source: None,
            image_default_source: None,
            video_default_source: None,
            system_llm_api_key_encrypted: None,
            system_image_api_key_encrypted: None,
            system_video_api_key_encrypted: None,
            llm_max_tokens: None,
            client_context: None,
        };

        let result = a4n_runtime::runtime_config::scope(snapshot.clone(), async {
            run_with_runtime_context(1, None, async move {
                let inherited = a4n_runtime::runtime_config::current_snapshot();
                assert_eq!(inherited, Some(snapshot));
                Ok(json!({"inherited": true}))
            })
            .await
        })
        .await
        .expect("detached command should complete");
        assert_eq!(result["inherited"], json!(true));
    }
}
