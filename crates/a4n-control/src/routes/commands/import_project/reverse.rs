use super::super::{
    ApiError, finish_web_task, import_reverse_actions, optional_string_field, register_web_task,
    sanitize_optional_task_id, strict_command_input, strict_optional_task_runtime_command_input,
    strict_runtime_command_input, to_value, web_progress_reporter,
};
use crate::state::ServerState;
use a4n_domain::task_queue::TaskKind;
use serde_json::Value;

pub(in crate::routes::commands) fn is_reverse_engineer_web_command(command: &str) -> bool {
    matches!(
        command,
        "reverse_engineer"
            | "preview_reverse_engineer_request"
            | "get_reverse_engineering_candidate"
            | "confirm_reverse_engineering_candidate"
    )
}

pub(in crate::routes::commands) async fn run_reverse_engineer_web_command(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    command: &str,
) -> Result<Value, ApiError> {
    match command {
        "reverse_engineer" => reverse_engineer_command(state, user_id, args).await,
        "preview_reverse_engineer_request" => {
            preview_reverse_engineer_request_command(state, user_id, args).await
        }
        "get_reverse_engineering_candidate" => {
            get_reverse_engineering_candidate_command(state, user_id, args).await
        }
        "confirm_reverse_engineering_candidate" => {
            confirm_reverse_engineering_candidate_command(state, user_id, args).await
        }
        _ => unreachable!("reverse engineer command was prefiltered"),
    }
}

async fn reverse_engineer_command(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: import_reverse_actions::ReverseEngineerInput =
        strict_optional_task_runtime_command_input(args, "reverse_engineer")?;
    let authorized =
        import_reverse_actions::authorize_reverse_engineer(state.runtime(), user_id, input).await?;
    let project_id = authorized.project_id();
    let target = authorized.target();
    let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
    let task = register_web_task(
        state,
        user_id,
        task_id.clone(),
        TaskKind::LlmCall,
        project_id,
        import_reverse_actions::REVERSE_ENGINEERING_PROGRESS_TOTAL,
        target.initial_message(),
    )?;
    let progress = web_progress_reporter(task.clone());
    let result = import_reverse_actions::reverse_engineer(
        state.runtime(),
        authorized,
        &progress,
        task_id.as_deref(),
    )
    .await;
    to_value(finish_web_task(state, task.as_ref(), result)?)
}

async fn preview_reverse_engineer_request_command(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: import_reverse_actions::ReverseEngineerInput =
        strict_runtime_command_input(args, "preview_reverse_engineer_request")?;
    let preview =
        import_reverse_actions::preview_reverse_engineer_request(state.runtime(), user_id, input)
            .await?;
    to_value(preview)
}

async fn get_reverse_engineering_candidate_command(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: import_reverse_actions::GetReverseEngineeringCandidateInput =
        strict_command_input(args, "get_reverse_engineering_candidate")?;
    let candidate =
        import_reverse_actions::get_reverse_engineering_candidate(state.runtime(), user_id, &input)
            .await?;
    to_value(candidate)
}

async fn confirm_reverse_engineering_candidate_command(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: import_reverse_actions::ConfirmReverseEngineeringCandidateInput =
        strict_runtime_command_input(args, "confirm_reverse_engineering_candidate")?;
    let candidate = import_reverse_actions::confirm_reverse_engineering_candidate(
        state.runtime(),
        user_id,
        &input,
    )
    .await?;
    to_value(candidate)
}
