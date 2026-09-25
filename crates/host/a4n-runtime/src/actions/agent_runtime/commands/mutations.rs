use super::super::*;

pub async fn wake_agent_external_task_wait(
    state: &AppState,
    request: &AgentWorkerExternalTaskWaitWakeRequest,
) -> Result<(), AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent external task wait wake 无效: {error}"))
    })?;
    state
        .agent_external_task_wait_application()
        .wake_external_task_wait(request.run_id, request.execution_id)
        .await
}

pub async fn cancel_agent_owned_generation_task(
    state: &AppState,
    user_id: i64,
    task_id: &str,
    attempt: u32,
) -> Result<(), AppError> {
    let candidate = state
        .agent_generation_cancel_application()
        .generation_cancel_candidates("")
        .await?
        .into_iter()
        .find(|task| task.task_id == task_id && task.attempt == attempt)
        .ok_or_else(|| AppError::Forbidden("生成任务不属于已停止的 Agent Run".into()))?;
    if candidate.requested_by_user_id != user_id
        || !candidate.owner_stopped
        || candidate.status == "cancelling"
    {
        return Err(AppError::Forbidden(
            "生成任务取消请求未通过 durable Agent owner 校验".into(),
        ));
    }
    crate::actions::generation_tasks::cancel_generation_task(
        state,
        candidate.requested_by_user_id,
        a4n_api_contract::generation_tasks::CancelGenerationTaskInput {
            task_id: candidate.task_id,
            attempt: candidate.attempt,
        },
    )
    .await
    .map(|_| ())
    .map_err(|error| AppError::InvalidArgument(error.to_string()))
}

pub async fn cancel_agent_owned_workflow(
    state: &AppState,
    workflow_run_id: i64,
    reason: &str,
) -> Result<(), AppError> {
    let candidate = state
        .agent_workflow_cancel_application()
        .workflow_cancel_candidates(workflow_run_id.saturating_sub(1))
        .await?
        .into_iter()
        .find(|workflow| workflow.workflow_run_id == workflow_run_id)
        .ok_or_else(|| AppError::Forbidden("Workflow 不属于 Agent durable owner".into()))?;
    if !candidate.owner_stopped {
        return Err(AppError::Forbidden(
            "只能取消已停止 Agent 所拥有的 Workflow".into(),
        ));
    }
    state
        .workflow_cancellation_application()
        .cancel(workflow_run_id, reason)
        .await
}

pub async fn ensure_main_thread(
    state: &AppState,
    user_id: i64,
    input: EnsureMainAgentThreadInput,
) -> Result<AgentThreadView, AppError> {
    state
        .agent_thread_mutation_application()
        .ensure_main_thread(user_id, input)
        .await
}

pub async fn create_thread(
    state: &AppState,
    user_id: i64,
    input: CreateAgentThreadInput,
) -> Result<AgentThreadView, AppError> {
    state
        .agent_thread_mutation_application()
        .create_thread(user_id, input)
        .await
}

pub async fn fork_thread(
    state: &AppState,
    user_id: i64,
    input: ForkAgentThreadInput,
) -> Result<AgentThreadView, AppError> {
    state
        .agent_thread_mutation_application()
        .fork_thread(user_id, input)
        .await
}

pub async fn update_thread(
    state: &AppState,
    user_id: i64,
    input: UpdateAgentThreadInput,
) -> Result<AgentThreadView, AppError> {
    state
        .agent_thread_mutation_application()
        .update_thread(user_id, input)
        .await
}

pub async fn archive_thread(
    state: &AppState,
    user_id: i64,
    input: ArchiveAgentThreadInput,
) -> Result<AgentThreadView, AppError> {
    state
        .agent_thread_mutation_application()
        .archive_thread(user_id, input)
        .await
}

pub async fn send_message(
    state: &AppState,
    user_id: i64,
    input: SendAgentMessageInput,
) -> Result<AgentMessageAccepted, AppError> {
    send_message_with_admission(state, user_id, input, false).await
}

pub async fn send_message_with_admission(
    state: &AppState,
    user_id: i64,
    mut input: SendAgentMessageInput,
    require_runtime_snapshot_for_new_run: bool,
) -> Result<AgentMessageAccepted, AppError> {
    let runtime_config_snapshot = input.runtime_config_snapshot.take();
    let runtime_snapshot_required = runtime_config_snapshot.is_some();
    let _admission = state.lock_agent_admission().await;
    crate::actions::package::ensure_package_execution_enabled(state, PackageId::Agent).await?;
    let accepted = state
        .agent_run_mutation_application()
        .send_message(
            user_id,
            input,
            runtime_snapshot_required,
            require_runtime_snapshot_for_new_run,
        )
        .await?;
    if accepted.created_run
        && let Some(runtime_config_snapshot) = runtime_config_snapshot
    {
        let run_id = accepted
            .run_id
            .parse::<i64>()
            .map_err(|_| AppError::InvalidState {
                current: format!("Agent accepted run id 无效: {}", accepted.run_id),
                operation: "绑定 Agent runtime snapshot".into(),
            })?;
        state
            .store_agent_runtime_snapshot(run_id, None, runtime_config_snapshot)
            .await;
    }
    state.trigger_agent_worker();
    Ok(accepted)
}

pub async fn control_run(
    state: &AppState,
    user_id: i64,
    input: ControlAgentRunInput,
) -> Result<AgentRunView, AppError> {
    let action = input.action;
    let result = state
        .agent_run_mutation_application()
        .control_run(user_id, input)
        .await?;
    if matches!(
        action,
        a4n_api_contract::agent::AgentControlAction::Cancel
            | a4n_api_contract::agent::AgentControlAction::Pause
    ) {
        let run_id = result
            .id
            .parse::<i64>()
            .map_err(|_| AppError::InvalidState {
                current: format!("Agent run id 无效: {}", result.id),
                operation: "取消 Agent runtime worker".into(),
            })?;
        state.cancel_agent_run_tree(run_id).await;
        if action == a4n_api_contract::agent::AgentControlAction::Cancel {
            state.remove_agent_runtime_snapshot_tree(run_id).await;
            task_wait::cancel_stopped_owned_tasks(state).await?;
        }
    }
    state.trigger_agent_worker();
    Ok(result)
}

pub async fn send_message_to_child(
    state: &AppState,
    user_id: i64,
    input: SendAgentMessageToChildInput,
) -> Result<AgentMessageAccepted, AppError> {
    send_message_to_child_with_admission(state, user_id, input, false).await
}

pub async fn send_message_to_child_with_admission(
    state: &AppState,
    user_id: i64,
    mut input: SendAgentMessageToChildInput,
    require_runtime_snapshot_for_new_run: bool,
) -> Result<AgentMessageAccepted, AppError> {
    let runtime_config_snapshot = input.runtime_config_snapshot.take();
    let runtime_snapshot_required = runtime_config_snapshot.is_some();
    let _admission = state.lock_agent_admission().await;
    crate::actions::package::ensure_package_execution_enabled(state, PackageId::Agent).await?;
    let accepted = state
        .agent_run_mutation_application()
        .send_message_to_child(
            user_id,
            input,
            runtime_snapshot_required,
            require_runtime_snapshot_for_new_run,
        )
        .await?;
    if accepted.created_run
        && let Some(runtime_config_snapshot) = runtime_config_snapshot
    {
        let run_id = accepted
            .run_id
            .parse::<i64>()
            .map_err(|_| AppError::InvalidState {
                current: format!("Agent accepted child run id 无效: {}", accepted.run_id),
                operation: "绑定 child Agent runtime snapshot".into(),
            })?;
        state
            .store_agent_runtime_snapshot(run_id, None, runtime_config_snapshot)
            .await;
    }
    state.trigger_agent_worker();
    Ok(accepted)
}

pub async fn control_child_agent(
    state: &AppState,
    user_id: i64,
    input: ControlChildAgentInput,
) -> Result<AgentRunView, AppError> {
    let action = input.action;
    let result = state
        .agent_run_mutation_application()
        .control_child_agent(user_id, input)
        .await?;
    if action == a4n_api_contract::agent::AgentControlAction::Cancel {
        let run_id = result
            .id
            .parse::<i64>()
            .map_err(|_| AppError::InvalidState {
                current: format!("Agent child run id 无效: {}", result.id),
                operation: "取消 child Agent runtime worker".into(),
            })?;
        state.cancel_agent_run_tree(run_id).await;
        state.remove_agent_runtime_snapshot_tree(run_id).await;
        task_wait::cancel_stopped_owned_tasks(state).await?;
    }
    state.trigger_agent_worker();
    Ok(result)
}

pub async fn spawn_agent(
    state: &AppState,
    user_id: i64,
    input: SpawnAgentInput,
) -> Result<a4n_api_contract::agent::AgentDelegationView, AppError> {
    spawn_agent_with_admission(state, user_id, input, false).await
}

/// Remote Hosts must bind a complete runtime snapshot before admitting a new
/// child Run. Desktop callers use the local Host path where provider
/// configuration is already resident in this process.
pub async fn spawn_agent_with_admission(
    state: &AppState,
    user_id: i64,
    mut input: SpawnAgentInput,
    require_runtime_snapshot_for_new_run: bool,
) -> Result<a4n_api_contract::agent::AgentDelegationView, AppError> {
    let parent_run_id = input
        .parent_run_id
        .parse::<i64>()
        .map_err(|_| AppError::InvalidArgument("parent_run_id 必须是正整数".into()))?;
    // A child created from a durable parent inherits the exact process-local
    // provider snapshot.  Remote callers may provide one explicitly; the
    // parent snapshot wins only when the request omitted it.
    if input.runtime_config_snapshot.is_none() {
        input.runtime_config_snapshot = state.agent_runtime_snapshot(parent_run_id).await;
    }
    let runtime_snapshot = input.runtime_config_snapshot.clone();
    if require_runtime_snapshot_for_new_run && runtime_snapshot.is_none() {
        return Err(AppError::InvalidArgument(
            "创建 Agent child run 必须提供 runtime_config_snapshot".into(),
        ));
    }
    let _admission = state.lock_agent_admission().await;
    crate::actions::package::ensure_package_execution_enabled(state, PackageId::Agent).await?;
    let result = state
        .agent_delegation_application()
        .spawn_agent(user_id, input)
        .await?;
    if let Some(runtime_snapshot) = runtime_snapshot {
        let child_run_id = result
            .child_run_id
            .as_deref()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("delegation {} 缺少 child_run_id", result.id),
                operation: "绑定 Agent child runtime snapshot".into(),
            })?
            .parse::<i64>()
            .map_err(|_| AppError::InvalidState {
                current: format!("delegation {} child_run_id 无效", result.id),
                operation: "绑定 Agent child runtime snapshot".into(),
            })?;
        state
            .store_agent_runtime_snapshot(child_run_id, Some(parent_run_id), runtime_snapshot)
            .await;
    }
    state.trigger_agent_worker();
    Ok(result)
}

pub async fn update_skill_configuration(
    state: &AppState,
    user_id: i64,
    input: a4n_api_contract::agent::UpdateAgentSkillConfigurationInput,
) -> Result<a4n_api_contract::agent::AgentSkillProfileView, AppError> {
    state
        .agent_skill_application()
        .update_skill_configuration(user_id, input)
        .await
}

pub async fn create_memory(
    state: &AppState,
    user_id: i64,
    input: AgentMemoryInput,
) -> Result<AgentMemoryView, AppError> {
    state
        .agent_memory_mutation_application()
        .create_memory(user_id, input)
        .await
}

pub async fn retract_memory(
    state: &AppState,
    user_id: i64,
    input: RetractAgentMemoryInput,
) -> Result<AgentMemoryView, AppError> {
    state
        .agent_memory_mutation_application()
        .retract_memory(user_id, input)
        .await
}

pub async fn review_delegation(
    state: &AppState,
    user_id: i64,
    input: a4n_api_contract::agent::ReviewAgentDelegationInput,
) -> Result<a4n_api_contract::agent::AgentDelegationView, AppError> {
    state
        .agent_delegation_application()
        .review_delegation(user_id, input, None)
        .await
}

pub async fn resolve_approval(
    state: &AppState,
    user_id: i64,
    input: ResolveAgentApprovalInput,
) -> Result<AgentApprovalView, AppError> {
    let result = state
        .agent_approval_application()
        .resolve_approval(user_id, input)
        .await?;
    state.trigger_agent_worker();
    Ok(result)
}
