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
