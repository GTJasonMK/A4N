use std::future::Future;

use serde_json::Value;

use super::RuntimeConfigSnapshot;

tokio::task_local! {
    pub(super) static RUNTIME_CONFIG_SNAPSHOT: RuntimeConfigSnapshot;
    pub(super) static RUNTIME_USER_ID: i64;
}

pub fn snapshot_from_args(args: &Value) -> Result<Option<RuntimeConfigSnapshot>, String> {
    let Some(value) = args.get("runtime_config_snapshot") else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let snapshot: RuntimeConfigSnapshot = serde_json::from_value(value.clone())
        .map_err(|err| format!("runtime_config_snapshot 无效: {err}"))?;
    Ok(Some(snapshot))
}

pub fn scope<F, T>(snapshot: RuntimeConfigSnapshot, future: F) -> impl Future<Output = T>
where
    F: Future<Output = T>,
{
    RUNTIME_CONFIG_SNAPSHOT.scope(snapshot, Box::pin(future))
}

pub fn request_scope<F, T>(
    user_id: i64,
    snapshot: Option<RuntimeConfigSnapshot>,
    future: F,
) -> impl Future<Output = T>
where
    F: Future<Output = T>,
{
    let future = Box::pin(future);
    RUNTIME_USER_ID.scope(user_id, async move {
        match snapshot {
            Some(snapshot) => RUNTIME_CONFIG_SNAPSHOT.scope(snapshot, future).await,
            None => future.await,
        }
    })
}

/// Capture before spawning: Tokio does not inherit task-local identity or secrets.
pub(crate) fn inherit_runtime_context<F: Future>(future: F) -> impl Future<Output = F::Output> {
    let user_id = super::current_user_id();
    let snapshot = super::current_snapshot();
    async move {
        match (user_id, snapshot) {
            (Some(user_id), snapshot) => request_scope(user_id, snapshot, future).await,
            (None, Some(snapshot)) => scope(snapshot, future).await,
            (None, None) => future.await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn detached_work_preserves_remote_identity_without_a_model_snapshot() {
        let (user_id, snapshot) = request_scope(7, None, async {
            tokio::spawn(inherit_runtime_context(async {
                (
                    super::super::current_user_id(),
                    super::super::current_snapshot(),
                )
            }))
            .await
            .unwrap()
        })
        .await;
        assert_eq!(user_id, Some(7));
        assert!(snapshot.is_none());
        assert!(super::super::current_user_id().is_none());
    }
}
