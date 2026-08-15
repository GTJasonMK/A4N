use super::super::{
    ApiError, finish_web_task, import_reverse_actions, register_web_task,
    sanitize_optional_task_id, to_value, web_progress_reporter,
};
use crate::state::ServerState;
use a4n_core::event::channel::TaskKind;
use serde::Deserialize;
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReverseEngineerCommandArgs {
    #[serde(default, rename = "runtime_config_snapshot")]
    _runtime_config_snapshot: Option<Value>,
    input: import_reverse_actions::ReverseEngineerInput,
    #[serde(default, rename = "taskId")]
    task_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeInputArgs<T> {
    #[serde(default, rename = "runtime_config_snapshot")]
    _runtime_config_snapshot: Option<Value>,
    input: T,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputArgs<T> {
    input: T,
}

async fn reverse_engineer_command(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let command: ReverseEngineerCommandArgs = serde_json::from_value(args.clone())
        .map_err(|error| ApiError::bad_request(format!("reverse_engineer 参数无效: {error}")))?;
    let authorized =
        import_reverse_actions::authorize_reverse_engineer(state.runtime(), user_id, command.input)
            .await?;
    let project_id = authorized.project_id();
    let target = authorized.target();
    let task_id = sanitize_optional_task_id(command.task_id)?;
    let task = register_web_task(
        state,
        user_id,
        task_id.clone(),
        TaskKind::LlmCall,
        project_id,
        import_reverse_actions::REVERSE_ENGINEERING_PROGRESS_TOTAL,
        target.initial_message(),
    );
    let progress = web_progress_reporter(task);
    let result = import_reverse_actions::reverse_engineer(
        state.runtime(),
        authorized,
        &progress,
        task_id.as_deref(),
    )
    .await;
    to_value(finish_web_task(state, task_id, result)?)
}

async fn preview_reverse_engineer_request_command(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let command: RuntimeInputArgs<import_reverse_actions::ReverseEngineerInput> =
        serde_json::from_value(args.clone()).map_err(|error| {
            ApiError::bad_request(format!(
                "preview_reverse_engineer_request 参数无效: {error}"
            ))
        })?;
    let preview = import_reverse_actions::preview_reverse_engineer_request(
        state.runtime(),
        user_id,
        command.input,
    )
    .await?;
    to_value(preview)
}

async fn get_reverse_engineering_candidate_command(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let command: InputArgs<import_reverse_actions::GetReverseEngineeringCandidateInput> =
        serde_json::from_value(args.clone()).map_err(|error| {
            ApiError::bad_request(format!(
                "get_reverse_engineering_candidate 参数无效: {error}"
            ))
        })?;
    let candidate = import_reverse_actions::get_reverse_engineering_candidate(
        state.runtime(),
        user_id,
        &command.input,
    )
    .await?;
    to_value(candidate)
}

async fn confirm_reverse_engineering_candidate_command(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let command: RuntimeInputArgs<import_reverse_actions::ConfirmReverseEngineeringCandidateInput> =
        serde_json::from_value(args.clone()).map_err(|error| {
            ApiError::bad_request(format!(
                "confirm_reverse_engineering_candidate 参数无效: {error}"
            ))
        })?;
    let candidate = import_reverse_actions::confirm_reverse_engineering_candidate(
        state.runtime(),
        user_id,
        &command.input,
    )
    .await?;
    to_value(candidate)
}

#[cfg(test)]
mod reverse_engineer_command_args_tests {
    use super::{ReverseEngineerCommandArgs, RuntimeInputArgs};
    use a4n_runtime::host_api::import_reverse::ReverseEngineerInput;
    use serde_json::{Value, json};

    fn reverse_input_json() -> Value {
        json!({
            "target": "outline",
            "project_id": 42,
            "content_sample": "样本文本",
            "auto_context": true,
            "context_limit": 8,
            "variables": null,
        })
    }

    #[test]
    fn accepts_runtime_envelope_fields_for_run_command() {
        let args: ReverseEngineerCommandArgs = serde_json::from_value(json!({
            "runtime_config_snapshot": { "schema_version": 1 },
            "input": reverse_input_json(),
            "taskId": "reverse-task-1",
        }))
        .expect("runtime envelope fields should be accepted");

        assert_eq!(args.input.project_id, 42);
        assert_eq!(args.task_id.as_deref(), Some("reverse-task-1"));
    }

    #[test]
    fn rejects_retired_user_identity_and_unknown_fields() {
        for field in ["userId", "unexpected"] {
            let error = serde_json::from_value::<RuntimeInputArgs<ReverseEngineerInput>>(json!({
                (field): 7,
                "input": reverse_input_json(),
            }))
            .expect_err("retired and unknown fields must remain explicit command errors");

            assert!(error.to_string().contains(field));
        }
    }
}
