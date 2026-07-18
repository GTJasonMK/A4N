//! 任务队列命令。

use a4n_runtime::AppState;
use a4n_runtime::host_api::task_records;
use task_records::{QueueStatsSummary, TaskInfoSummary};
use tauri::State;

#[tauri::command]
pub async fn cancel_task(
    state: State<'_, AppState>,
    user_id: i64,
    task_id: String,
) -> Result<(), String> {
    task_records::cancel_task(&state, user_id, &task_id).await
}

#[tauri::command]
pub async fn get_queue_stats(
    state: State<'_, AppState>,
    user_id: i64,
) -> Result<QueueStatsSummary, String> {
    task_records::get_queue_stats(&state, user_id).await
}

#[tauri::command]
pub async fn get_task_history(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: Option<i64>,
    offset: u64,
    limit: u64,
) -> Result<Vec<TaskInfoSummary>, String> {
    task_records::get_task_history(&state, user_id, project_id, offset, limit).await
}
