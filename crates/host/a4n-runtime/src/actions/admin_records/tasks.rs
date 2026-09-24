use super::TaskQueueSummary;
use crate::AppState;
use serde_json::{Value, json};

pub(super) async fn list_pending_tasks(
    state: &AppState,
    caller_user_id: i64,
) -> Result<Vec<TaskQueueSummary>, String> {
    state
        .admin_read_application()
        .pending_tasks(caller_user_id)
        .await
        .map_err(|error| error.to_string())
}

pub(super) async fn cancel_admin_task(state: &AppState, task_id: &str) -> Result<(), String> {
    state
        .task_cancellation_application()
        .cancel(task_id)
        .await
        .map_err(|error| error.to_string())
}

pub(super) async fn health_check(state: &AppState, caller_user_id: i64) -> Result<Value, String> {
    state
        .admin_read_application()
        .check_database_health(caller_user_id)
        .await
        .map_err(|error| error.to_string())?;
    let services = state
        .service_health_application()
        .summaries()
        .await
        .map_err(|error| error.to_string())?;
    Ok(json!({
        "runtime_profile": state.runtime_profile().as_str(),
        "services": services,
    }))
}
