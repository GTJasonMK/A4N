use serde_json::Value;

use super::super::{
    ApiError, ServerState, command_input, non_negative_u64_arg, nullable_i64_arg, string_arg,
    task_records_actions, to_value, u64_arg,
};
use a4n_api_contract::generation_tasks::CancelGenerationTaskInput;
use a4n_runtime::host_api::generation_tasks as generation_task_actions;

pub(in crate::routes::commands) async fn list_active_workflows(
    state: &ServerState,
    user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
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
    let input: task_records_actions::ListWorkflowRunsInput =
        command_input(args, "list_workflow_runs")?;
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
    let input: task_records_actions::GetWorkflowRunInput =
        command_input(args, "get_workflow_run_detail")?;
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
    let input: task_records_actions::ResumeWorkflowRunInput =
        command_input(args, "resume_workflow_run")?;
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
    let input: task_records_actions::WorkflowProgressEventsInput =
        command_input(args, "get_workflow_progress_events")?;
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
    let input: task_records_actions::CancelWorkflowInput = command_input(args, "cancel_workflow")?;
    task_records_actions::cancel_workflow(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn cancel_task(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let task_id = string_arg(args, "taskId")?;
    task_records_actions::cancel_task(state.runtime(), user_id, &task_id)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn cancel_generation_task(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: CancelGenerationTaskInput = command_input(args, "cancel_generation_task")?;
    let handle = generation_task_actions::cancel_generation_task(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(handle)
}

pub(in crate::routes::commands) async fn get_queue_stats(
    state: &ServerState,
    user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
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
    let project_id = nullable_i64_arg(args, "projectId")?;
    let offset = non_negative_u64_arg(args, "offset")?;
    let limit = u64_arg(args, "limit")?;
    let history =
        task_records_actions::get_task_history(state.runtime(), user_id, project_id, offset, limit)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(history)
}
