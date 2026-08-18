use serde_json::Value;

use super::super::{
    ApiError, ServerState, empty_command_args, strict_command_input, strict_runtime_command_input,
    task_records_actions, to_value,
};
use a4n_api_contract::generation_tasks::CancelGenerationTaskInput;
use a4n_api_contract::{
    task::{CancelTaskInput, GetTaskHistoryInput},
    workflow::{
        CancelWorkflowInput, GetWorkflowRunInput, ListWorkflowRunsInput, ResumeWorkflowRunInput,
        WorkflowProgressEventsInput,
    },
};
use a4n_runtime::host_api::generation_tasks as generation_task_actions;

pub(in crate::routes::commands) async fn list_active_workflows(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    empty_command_args(args, "list_active_workflows")?;
    let active = task_records_actions::list_active_workflows_for_user(state.runtime(), user_id)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(active)
}

pub(in crate::routes::commands) async fn list_workflow_runs(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: ListWorkflowRunsInput = strict_command_input(args, "list_workflow_runs")?;
    let runs = task_records_actions::list_workflow_runs_for_user(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(runs)
}

pub(in crate::routes::commands) async fn get_workflow_run_detail(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: GetWorkflowRunInput = strict_command_input(args, "get_workflow_run_detail")?;
    let detail =
        task_records_actions::get_workflow_run_detail_for_user(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(detail)
}

pub(in crate::routes::commands) async fn resume_workflow_run(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: ResumeWorkflowRunInput = strict_runtime_command_input(args, "resume_workflow_run")?;
    let run = task_records_actions::resume_workflow_run_for_user(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(run)
}

pub(in crate::routes::commands) async fn get_workflow_progress_events(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: WorkflowProgressEventsInput =
        strict_command_input(args, "get_workflow_progress_events")?;
    let events =
        task_records_actions::get_workflow_progress_events(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(events)
}

pub(in crate::routes::commands) async fn cancel_workflow(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: CancelWorkflowInput = strict_command_input(args, "cancel_workflow")?;
    task_records_actions::cancel_workflow(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(())
}

pub(in crate::routes::commands) async fn cancel_task(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = serde_json::from_value::<CancelTaskInput>(args.clone())
        .map_err(|error| ApiError::bad_request(format!("cancel_task 参数无效: {error}")))?;
    task_records_actions::cancel_task(state.runtime(), user_id, &input.task_id)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(())
}

pub(in crate::routes::commands) async fn cancel_generation_task(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: CancelGenerationTaskInput = strict_command_input(args, "cancel_generation_task")?;
    let handle = generation_task_actions::cancel_generation_task(state.runtime(), user_id, input)
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    to_value(handle)
}

pub(in crate::routes::commands) async fn get_queue_stats(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    empty_command_args(args, "get_queue_stats")?;
    let stats = task_records_actions::get_queue_stats(state.runtime(), user_id)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(stats)
}

pub(in crate::routes::commands) async fn get_task_history(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = serde_json::from_value::<GetTaskHistoryInput>(args.clone())
        .map_err(|error| ApiError::bad_request(format!("get_task_history 参数无效: {error}")))?;
    let history = task_records_actions::get_task_history(
        state.runtime(),
        user_id,
        input.project_id,
        input.offset,
        input.limit,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(history)
}
