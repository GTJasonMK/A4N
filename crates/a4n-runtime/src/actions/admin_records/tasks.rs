use super::TaskQueueSummary;
use crate::{AppState, actions::app_error_message};
use serde_json::{Value, json};

pub async fn list_pending_tasks(state: &AppState) -> Result<Vec<TaskQueueSummary>, String> {
    state
        .orchestrator()
        .pending_tasks()
        .await
        .map_err(app_error_message)
}

pub async fn cancel_admin_task(state: &AppState, task_id: &str) -> Result<(), String> {
    state
        .orchestrator()
        .queue_cancel(task_id)
        .await
        .map_err(|error| error.to_string())
}

pub async fn health_check(state: &AppState) -> Result<Value, String> {
    state
        .orchestrator()
        .check_database_health()
        .await
        .map_err(app_error_message)?;
    let services = state
        .orchestrator()
        .managed_service_health_summaries()
        .await
        .map_err(app_error_message)?;
    Ok(json!({
        "runtime_profile": state.runtime_profile().as_str(),
        "services": services,
    }))
}
