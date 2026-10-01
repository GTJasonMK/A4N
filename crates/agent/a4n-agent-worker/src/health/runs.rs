use a4n_agent_runtime_contract::AgentRunRuntimeCommand;

use super::*;

pub(super) async fn show_run(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_RUN_SHOW_OPERATION,
        |app, user_id, payload| async move {
            let run_id: String = decode_payload(payload)?;
            app.show_run(user_id, &run_id).await
        },
    )
    .await
}

pub(super) async fn list_events(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_EVENT_LIST_OPERATION,
        |app, user_id, payload| async move { app.events(user_id, decode_payload(payload)?).await },
    )
    .await
}

pub(super) async fn send_message(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_RUN_SEND_OPERATION,
        |app, user_id, payload| async move {
            let command: AgentRunRuntimeCommand<Value> = decode_payload(payload)?;
            app.send_message(
                user_id,
                decode_payload(command.input)?,
                command.runtime_snapshot_required,
                command.require_runtime_snapshot_for_new_run,
            )
            .await
        },
    )
    .await
}

pub(super) async fn control_run(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    let runtime = state.runtime.clone();
    user_command(
        &state,
        &headers,
        request,
        AGENT_RUN_CONTROL_OPERATION,
        |app, user_id, payload| async move {
            let input: a4n_api_contract::agent::ControlAgentRunInput = decode_payload(payload)?;
            let action = input.action;
            let result = app.control_run(user_id, input).await?;
            apply_runtime_control(runtime.as_ref(), &result, action).await?;
            Ok(result)
        },
    )
    .await
}

pub(super) async fn send_message_to_child(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_RUN_SEND_CHILD_OPERATION,
        |app, user_id, payload| async move {
            let command: AgentRunRuntimeCommand<Value> = decode_payload(payload)?;
            app.send_message_to_child(
                user_id,
                decode_payload(command.input)?,
                command.runtime_snapshot_required,
                command.require_runtime_snapshot_for_new_run,
            )
            .await
        },
    )
    .await
}

pub(super) async fn control_child(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    let runtime = state.runtime.clone();
    user_command(
        &state,
        &headers,
        request,
        AGENT_RUN_CONTROL_CHILD_OPERATION,
        |app, user_id, payload| async move {
            let input: a4n_api_contract::agent::ControlChildAgentInput = decode_payload(payload)?;
            let action = input.action;
            let result = app.control_child_agent(user_id, input).await?;
            apply_runtime_control(runtime.as_ref(), &result, action).await?;
            Ok(result)
        },
    )
    .await
}

async fn apply_runtime_control(
    runtime: &a4n_agent_runtime::AgentRuntimeState,
    result: &a4n_api_contract::agent::AgentRunView,
    action: a4n_api_contract::agent::AgentControlAction,
) -> Result<(), AppError> {
    let run_id = result
        .id
        .parse::<i64>()
        .map_err(|_| AppError::InvalidArgument("Agent Run ID 无效".into()))?;
    if matches!(
        action,
        a4n_api_contract::agent::AgentControlAction::Pause
            | a4n_api_contract::agent::AgentControlAction::Cancel
    ) {
        runtime.cancel_agent_run_tree(run_id).await;
    }
    if action == a4n_api_contract::agent::AgentControlAction::Cancel {
        runtime.remove_agent_runtime_snapshot_tree(run_id).await;
    }
    runtime.trigger_agent_worker();
    Ok(())
}
