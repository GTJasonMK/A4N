use super::super::{
    ApiError, finish_web_task, import_reverse_actions, register_web_task,
    sanitize_optional_task_id, to_value, web_progress_reporter,
};
use crate::state::ServerState;
use a4n_core::event::channel::TaskKind;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub(in crate::routes::commands) enum ReverseEngineerWebCommand {
    Run {
        target: import_reverse_actions::ReverseEngineeringTarget,
    },
    Preview {
        target: import_reverse_actions::ReverseEngineeringTarget,
    },
    ConfirmCandidate,
}

pub(in crate::routes::commands) fn reverse_engineer_web_command(
    command: &str,
) -> Option<ReverseEngineerWebCommand> {
    use ReverseEngineerWebCommand::{ConfirmCandidate, Preview, Run};
    use import_reverse_actions::ReverseEngineeringTarget::{Blueprint, Outline, PartOutline};

    Some(match command {
        "reverse_engineer_blueprint" => Run { target: Blueprint },
        "preview_reverse_engineer_blueprint_request" => Preview { target: Blueprint },
        "reverse_engineer_outline" => Run { target: Outline },
        "preview_reverse_engineer_outline_request" => Preview { target: Outline },
        "reverse_engineer_part_outline" => Run {
            target: PartOutline,
        },
        "preview_reverse_engineer_part_outline_request" => Preview {
            target: PartOutline,
        },
        "confirm_reverse_engineering_candidate" => ConfirmCandidate,
        _ => return None,
    })
}

pub(in crate::routes::commands) async fn run_reverse_engineer_web_command(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    command: ReverseEngineerWebCommand,
) -> Result<Value, ApiError> {
    match command {
        ReverseEngineerWebCommand::Run { target } => {
            reverse_engineer_command(state, user_id, args, target).await
        }
        ReverseEngineerWebCommand::Preview { target } => {
            preview_reverse_engineer_request_command(state, user_id, args, target).await
        }
        ReverseEngineerWebCommand::ConfirmCandidate => {
            confirm_reverse_engineering_candidate_command(state, user_id, args).await
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReverseEngineerCommandArgs {
    #[serde(default, rename = "userId")]
    _user_id: Option<i64>,
    #[serde(default, rename = "runtime_config_snapshot")]
    _runtime_config_snapshot: Option<Value>,
    input: import_reverse_actions::ReverseEngineerInput,
    #[serde(default, rename = "taskId")]
    task_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReverseEngineerPreviewArgs {
    #[serde(default, rename = "userId")]
    _user_id: Option<i64>,
    #[serde(default, rename = "runtime_config_snapshot")]
    _runtime_config_snapshot: Option<Value>,
    input: import_reverse_actions::ReverseEngineerInput,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfirmReverseEngineerCandidateArgs {
    #[serde(default, rename = "userId")]
    _user_id: Option<i64>,
    #[serde(default, rename = "runtime_config_snapshot")]
    _runtime_config_snapshot: Option<Value>,
    input: import_reverse_actions::ConfirmReverseEngineeringCandidateInput,
}

pub(in crate::routes::commands) async fn reverse_engineer_command(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    target: import_reverse_actions::ReverseEngineeringTarget,
) -> Result<Value, ApiError> {
    let command: ReverseEngineerCommandArgs = serde_json::from_value(args.clone())
        .map_err(|e| ApiError::bad_request(format!("{} 参数无效: {e}", target.prompt_name())))?;
    let scope = import_reverse_actions::reverse_engineer_task_scope(
        state.runtime(),
        user_id,
        &command.input,
    )
    .await?;

    let task_id = sanitize_optional_task_id(command.task_id)?;
    let task = register_web_task(
        state,
        user_id,
        task_id.clone(),
        TaskKind::LlmCall,
        scope.project_id,
        scope.total,
        target.initial_message(),
    );
    let progress = web_progress_reporter(task);
    let result = import_reverse_actions::reverse_engineer(
        state.runtime(),
        user_id,
        &command.input,
        target,
        &progress,
        task_id.as_deref(),
    )
    .await;
    finish_web_task(state, task_id, result)
}

pub(in crate::routes::commands) async fn preview_reverse_engineer_request_command(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    target: import_reverse_actions::ReverseEngineeringTarget,
) -> Result<Value, ApiError> {
    let command: ReverseEngineerPreviewArgs =
        serde_json::from_value(args.clone()).map_err(|e| {
            ApiError::bad_request(format!(
                "preview_{}_request 参数无效: {e}",
                target.prompt_name()
            ))
        })?;
    let preview = import_reverse_actions::preview_reverse_engineer_request(
        state.runtime(),
        user_id,
        &command.input,
        target,
    )
    .await?;
    to_value(preview)
}

async fn confirm_reverse_engineering_candidate_command(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let command: ConfirmReverseEngineerCandidateArgs = serde_json::from_value(args.clone())
        .map_err(|e| {
            ApiError::bad_request(format!(
                "confirm_reverse_engineering_candidate 参数无效: {e}"
            ))
        })?;
    import_reverse_actions::confirm_reverse_engineering_candidate(
        state.runtime(),
        user_id,
        &command.input,
    )
    .await
    .map_err(|error| ApiError::bad_request(error.to_string()))
}

#[cfg(test)]
mod reverse_engineer_command_args_tests {
    use super::{ReverseEngineerCommandArgs, ReverseEngineerPreviewArgs};
    use serde_json::{Value, json};

    pub(in crate::routes::commands) fn reverse_input_json() -> Value {
        json!({
            "project_id": 42,
            "content_sample": "样本文本",
            "auto_context": true,
            "context_limit": 8,
            "variables": null,
        })
    }

    #[test]
    fn accepts_web_envelope_fields_for_run_command() {
        let args: ReverseEngineerCommandArgs = serde_json::from_value(json!({
            "userId": 7,
            "runtime_config_snapshot": { "schema_version": 1 },
            "input": reverse_input_json(),
            "taskId": "reverse-task-1",
        }))
        .expect("web envelope fields should be accepted");

        assert_eq!(args.input.project_id, 42);
        assert_eq!(args.task_id.as_deref(), Some("reverse-task-1"));
    }

    #[test]
    fn accepts_web_envelope_fields_for_preview_command() {
        let args: ReverseEngineerPreviewArgs = serde_json::from_value(json!({
            "userId": 7,
            "runtime_config_snapshot": { "schema_version": 1 },
            "input": reverse_input_json(),
        }))
        .expect("web envelope fields should be accepted");

        assert_eq!(args.input.project_id, 42);
    }

    #[test]
    fn rejects_unknown_reverse_command_fields() {
        let err = serde_json::from_value::<ReverseEngineerPreviewArgs>(json!({
            "userId": 7,
            "input": reverse_input_json(),
            "unexpected": true,
        }))
        .expect_err("unknown fields should remain explicit command errors");

        assert!(err.to_string().contains("unexpected"));
    }
}
