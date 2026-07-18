//! 流式工作流命令。

use a4n_runtime::AppState;
use a4n_runtime::host_api::task_records;
use task_records::{
    CancelWorkflowInput, GetWorkflowRunInput, ListWorkflowRunsInput, ResumeWorkflowRunInput,
    TaskProgressEventSummary, WorkflowProgressEventsInput, WorkflowRunDetailSummary,
    WorkflowRunSummary, WorkflowStateSummary,
};
use tauri::State;

#[tauri::command]
pub async fn list_active_workflows(
    state: State<'_, AppState>,
    user_id: i64,
) -> Result<Vec<WorkflowStateSummary>, String> {
    task_records::list_active_workflows_for_user(&state, user_id).await
}

#[tauri::command]
pub async fn list_workflow_runs(
    state: State<'_, AppState>,
    user_id: i64,
    input: ListWorkflowRunsInput,
) -> Result<Vec<WorkflowRunSummary>, String> {
    task_records::list_workflow_runs_for_user(&state, user_id, input).await
}

#[tauri::command]
pub async fn get_workflow_run_detail(
    state: State<'_, AppState>,
    user_id: i64,
    input: GetWorkflowRunInput,
) -> Result<WorkflowRunDetailSummary, String> {
    task_records::get_workflow_run_detail_for_user(&state, user_id, input).await
}

#[tauri::command]
pub async fn resume_workflow_run(
    state: State<'_, AppState>,
    user_id: i64,
    input: ResumeWorkflowRunInput,
) -> Result<WorkflowRunSummary, String> {
    task_records::resume_workflow_run_for_user(&state, user_id, input).await
}

#[tauri::command]
pub async fn cancel_workflow(
    state: State<'_, AppState>,
    user_id: i64,
    input: CancelWorkflowInput,
) -> Result<(), String> {
    task_records::cancel_workflow(&state, user_id, input).await
}

#[tauri::command]
pub async fn get_workflow_progress_events(
    state: State<'_, AppState>,
    user_id: i64,
    input: WorkflowProgressEventsInput,
) -> Result<Vec<TaskProgressEventSummary>, String> {
    task_records::get_workflow_progress_events(&state, user_id, input).await
}
