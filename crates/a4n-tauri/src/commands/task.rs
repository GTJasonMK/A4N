//! 任务队列命令。

use a4n_api_contract::generation_tasks::{
    CancelGenerationTaskInput, GenerationTaskHandle, HostErrorResponse,
};
use a4n_runtime::AppState;
use a4n_runtime::host_api::{generation_tasks, task_records};
use a4n_service_protocol::GenerationErrorCode;
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
pub async fn cancel_generation_task(
    state: State<'_, AppState>,
    user_id: i64,
    input: CancelGenerationTaskInput,
) -> Result<GenerationTaskHandle, HostErrorResponse> {
    generation_tasks::cancel_generation_task_typed(&state, user_id, input)
        .await
        .map_err(|error| HostErrorResponse {
            code: GenerationErrorCode::registered(error.code())
                .expect("generation cancel error code must be registered"),
            message: error.to_string(),
            retryable: error.code() == "generation_terminal_persistence_unavailable",
            trace_id: None,
        })
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
