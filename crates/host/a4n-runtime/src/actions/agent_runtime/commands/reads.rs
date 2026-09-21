use super::super::*;

pub async fn list_agent_external_task_waits(
    state: &AppState,
    request: &AgentWorkerExternalTaskWaitScanRequest,
) -> Result<Vec<AgentWorkerExternalTaskWaitSnapshotWire>, AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent external task wait scan 无效: {error}"))
    })?;
    state
        .agent_external_task_wait_application()
        .external_task_waits(request.after_run_id)
        .await
        .map(|waits| {
            waits
                .into_iter()
                .map(|wait| AgentWorkerExternalTaskWaitSnapshotWire {
                    run_id: wait.run_id,
                    owner_user_id: wait.owner_user_id,
                    scope: wait.scope,
                    execution_id: wait.execution_id,
                    tool_name: wait.tool_name,
                    arguments_json: wait.arguments_json,
                    duration_exhausted: wait.duration_exhausted,
                })
                .collect()
        })
}

pub async fn list_agent_generation_cancel_candidates(
    state: &AppState,
    request: &AgentWorkerGenerationCancelScanRequest,
) -> Result<Vec<AgentWorkerGenerationCancelSnapshotWire>, AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent generation cancel scan 无效: {error}"))
    })?;
    state
        .agent_generation_cancel_application()
        .generation_cancel_candidates(&request.after_task_id)
        .await
        .map(|tasks| {
            tasks
                .into_iter()
                .map(|task| AgentWorkerGenerationCancelSnapshotWire {
                    task_id: task.task_id,
                    attempt: task.attempt,
                    requested_by_user_id: task.requested_by_user_id,
                    status: task.status,
                    owner_stopped: task.owner_stopped,
                })
                .collect()
        })
}

pub async fn list_agent_workflow_cancel_candidates(
    state: &AppState,
    request: &AgentWorkerWorkflowCancelScanRequest,
) -> Result<Vec<AgentWorkerWorkflowCancelSnapshotWire>, AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent workflow cancel scan 无效: {error}"))
    })?;
    state
        .agent_workflow_cancel_application()
        .workflow_cancel_candidates(request.after_workflow_run_id)
        .await
        .map(|workflows| {
            workflows
                .into_iter()
                .map(|workflow| AgentWorkerWorkflowCancelSnapshotWire {
                    workflow_run_id: workflow.workflow_run_id,
                    owner_stopped: workflow.owner_stopped,
                })
                .collect()
        })
}

pub async fn read_external_task_wait(
    state: &AppState,
    requested: AgentExternalTaskWaitSnapshot,
    runtime_snapshot: Option<crate::runtime_config::RuntimeConfigSnapshot>,
) -> Result<Value, AppError> {
    let authoritative = state
        .agent_external_task_wait_application()
        .external_task_wait(requested.run_id, requested.execution_id)
        .await?
        .ok_or_else(|| AppError::Forbidden("Agent wait 不再属于可访问的 durable Run".into()))?;
    let owner_matches = authoritative.owner_user_id == requested.owner_user_id;
    // Restricted capabilities hydrate their default tool catalog while the
    // worker builds AgentRunContext. The durable thread scope intentionally
    // stores an empty allowlist in that case; compare the resource fence and
    // capability policy, and require an explicit stored allowlist to match.
    let scope_matches = authoritative.scope.resource_kind == requested.scope.resource_kind
        && authoritative.scope.resource_id == requested.scope.resource_id
        && authoritative.scope.project_id == requested.scope.project_id
        && authoritative.scope.chapter_id == requested.scope.chapter_id
        && authoritative.scope.allowed_capabilities == requested.scope.allowed_capabilities
        && (authoritative.scope.allowed_tools.is_empty()
            || authoritative.scope.allowed_tools == requested.scope.allowed_tools);
    let tool_matches = authoritative.tool_name == requested.tool_name;
    let arguments_match = authoritative.arguments_json == requested.arguments_json;
    if !(owner_matches && scope_matches && tool_matches && arguments_match) {
        tracing::warn!(
            run_id = requested.run_id,
            execution_id = requested.execution_id,
            owner_matches,
            scope_matches,
            tool_matches,
            arguments_match,
            "Agent wait request 与 durable execution 不匹配"
        );
        return Err(AppError::Forbidden(
            "Agent wait 请求与 durable Run/thread scope 不匹配".into(),
        ));
    }
    task_wait::read_with_snapshot(
        state,
        authoritative.owner_user_id,
        &authoritative.scope,
        &authoritative,
        runtime_snapshot.as_ref(),
    )
    .await
}

pub async fn agent_llm_capabilities(
    state: &AppState,
    user_id: i64,
    runtime_snapshot: Option<crate::runtime_config::RuntimeConfigSnapshot>,
    config_id: Option<i64>,
) -> Result<a4n_agent_runtime_contract::AgentLlmCapabilities, AppError> {
    crate::runtime_config::request_scope(user_id, runtime_snapshot, async {
        let provider = state.dynamic_llm_provider_for_profile()?;
        Ok(a4n_agent_runtime_contract::AgentLlmCapabilities {
            model: provider.capabilities_for_config_id(config_id).await?,
            agent_protocol: provider.agent_protocol_for_config_id(config_id).await?,
        })
    })
    .await
}

pub async fn agent_chat_stream(
    state: &AppState,
    user_id: i64,
    runtime_snapshot: Option<crate::runtime_config::RuntimeConfigSnapshot>,
    config_id: Option<i64>,
    request: ChatRequest,
) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
    crate::runtime_config::request_scope(user_id, runtime_snapshot, async {
        state
            .dynamic_llm_provider_for_profile()?
            .chat_stream_with_config_id(config_id, request)
            .await
    })
    .await
}

pub async fn prepare_agent_chat_request(
    state: &AppState,
    user_id: i64,
    runtime_snapshot: Option<crate::runtime_config::RuntimeConfigSnapshot>,
    config_id: Option<i64>,
    request: ChatRequest,
) -> Result<ChatRequest, AppError> {
    crate::runtime_config::request_scope(user_id, runtime_snapshot, async {
        state
            .dynamic_llm_provider_for_profile()?
            .prepare_chat_request_with_config_id(config_id, request)
            .await
    })
    .await
}

pub async fn list_threads(
    state: &AppState,
    user_id: i64,
    input: ListAgentThreadsInput,
) -> Result<Vec<AgentThreadView>, AppError> {
    state
        .agent_thread_read_application()
        .list_threads(user_id, input)
        .await
}

pub async fn show_thread(
    state: &AppState,
    user_id: i64,
    thread_id: &str,
) -> Result<AgentThreadSnapshot, AppError> {
    state
        .agent_thread_read_application()
        .show_thread(user_id, thread_id)
        .await
}

pub async fn list_messages(
    state: &AppState,
    user_id: i64,
    input: ListAgentMessagesInput,
) -> Result<Vec<a4n_api_contract::agent::AgentMessageView>, AppError> {
    state
        .agent_thread_read_application()
        .list_messages(user_id, input)
        .await
}

pub async fn show_run(
    state: &AppState,
    user_id: i64,
    run_id: &str,
) -> Result<AgentRunView, AppError> {
    state
        .agent_run_read_application()
        .show_run(user_id, run_id)
        .await
}

pub async fn events(
    state: &AppState,
    user_id: i64,
    input: AgentEventsInput,
) -> Result<Vec<AgentEvent>, AppError> {
    state
        .agent_event_read_application()
        .events(user_id, input)
        .await
}

pub async fn search_tools(
    state: &AppState,
    user_id: i64,
    input: SearchAgentToolsInput,
) -> Result<AgentToolSearchResult, AppError> {
    state
        .agent_tool_read_application()
        .search_tools(user_id, input)
        .await
}

pub async fn describe_tool(
    state: &AppState,
    user_id: i64,
    input: DescribeAgentToolInput,
) -> Result<AgentToolDescriptor, AppError> {
    state
        .agent_tool_read_application()
        .describe_tool(user_id, input)
        .await
}

pub async fn list_skill_settings(
    state: &AppState,
    user_id: i64,
) -> Result<a4n_api_contract::agent::AgentSkillSettingsView, AppError> {
    state
        .agent_skill_application()
        .list_skill_settings(user_id)
        .await
}

pub async fn search_agent_skills_for_worker(
    state: &AppState,
    request: &AgentWorkerSkillSearchRequest,
) -> Result<AgentSkillSearchResult, AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent Skill search 无效: {error}")))?;
    state
        .agent_worker_read_application()
        .search_skills_for_worker(request)
        .await
}

pub async fn load_agent_skill_for_worker(
    state: &AppState,
    request: &AgentWorkerSkillLoadRequest,
) -> Result<AgentSkillContent, AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent Skill load 无效: {error}")))?;
    state
        .agent_worker_read_application()
        .load_skill_for_worker(request)
        .await
}

pub async fn list_memories(
    state: &AppState,
    user_id: i64,
    scope_kind: Option<AgentMemoryScopeKind>,
    scope_id: Option<String>,
) -> Result<Vec<AgentMemoryView>, AppError> {
    state
        .agent_memory_read_application()
        .list_memories(user_id, scope_kind, scope_id)
        .await
}
