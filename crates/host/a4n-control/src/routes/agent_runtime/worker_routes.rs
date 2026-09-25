pub(super) async fn claim_worker(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerClaimRequest>,
) -> Result<Response, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    match agent_actions::claim_agent_worker_request(state.runtime(), &input.worker_id).await? {
        Some(request) => Ok(Json(request).into_response()),
        None => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}

pub(super) async fn load_agent_context(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<AgentRunPayload>>,
) -> Result<Json<AgentWorkerRunContextWire>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    request
        .validate_agent_run()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::load_agent_run_context(state.runtime(), &request)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn renew_agent_lease(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerRunLeaseRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::renew_agent_worker_lease(state.runtime(), &input).await?;
    Ok(Json(Value::Null))
}

pub(super) async fn activate_agent_turn(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerRunLeaseRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::mark_agent_turn_active(state.runtime(), &input).await?;
    Ok(Json(Value::Null))
}

pub(super) async fn update_agent_usage(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerUsageRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::update_agent_worker_usage(state.runtime(), &input).await?;
    Ok(Json(Value::Null))
}

pub(super) async fn wait_agent_run(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerRunWaitRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::wait_agent_worker_run(state.runtime(), &input).await?;
    Ok(Json(Value::Null))
}

pub(super) async fn update_agent_context_summary(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerContextSummaryRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::update_agent_context_summary(state.runtime(), &input).await?;
    Ok(Json(Value::Null))
}

pub(super) async fn worker_execution_by_key(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerExecutionLookupRequest>,
) -> Result<Json<Option<AgentWorkerExecutionSnapshotWire>>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::worker_execution_by_key(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn create_prepared_execution(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerExecutionCreateRequest>,
) -> Result<Json<Option<AgentWorkerExecutionSnapshotWire>>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::create_prepared_execution(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn update_worker_execution(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerExecutionUpdateRequest>,
) -> Result<Json<AgentWorkerExecutionSnapshotWire>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::update_worker_execution(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn read_execution_result(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerExecutionResultRequest>,
) -> Result<Json<a4n_api_contract::agent::AgentExecutionResultPage>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::read_execution_result(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn read_memory(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerMemoryRequest>,
) -> Result<Json<a4n_api_contract::agent::AgentMemorySearchResult>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::read_memory(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn search_agent_tools(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerToolSearchRequest>,
) -> Result<Json<AgentToolSearchResult>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::search_agent_tools_for_worker(state.runtime(), &input)
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn describe_agent_tool(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerToolDescribeRequest>,
) -> Result<Json<AgentToolDescriptor>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::describe_agent_tool_for_worker(state.runtime(), &input)
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn search_agent_skills(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerSkillSearchRequest>,
) -> Result<Json<AgentSkillSearchResult>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::search_agent_skills_for_worker(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn load_agent_skill(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerSkillLoadRequest>,
) -> Result<Json<AgentSkillContent>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::load_agent_skill_for_worker(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn verify_agent_chapters(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerChapterScopeRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::verify_agent_chapters_for_worker(state.runtime(), &input).await?;
    Ok(Json(Value::Null))
}

pub(super) async fn read_worker_history(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerHistoryRequest>,
) -> Result<Json<AgentWorkerHistorySnapshotWire>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::read_worker_history(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn mark_agent_worker_recovery_required(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerRecoveryRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::mark_agent_worker_recovery_required(state.runtime(), &input).await?;
    Ok(Json(Value::Null))
}

pub(super) async fn wait_for_agent_editor(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerEditorWaitRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::wait_for_agent_editor(state.runtime(), &input).await?;
    Ok(Json(Value::Null))
}

pub(super) async fn expire_agent_editor_waits(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerEditorWaitExpiryRequest>,
) -> Result<Json<usize>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::expire_agent_editor_waits(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn expire_agent_approvals(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerApprovalExpiryRequest>,
) -> Result<Json<usize>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::expire_agent_approvals(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn recover_expired_agent_runs(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerExpiredRunRecoveryRequest>,
) -> Result<Json<usize>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::recover_expired_agent_runs(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn list_agent_external_task_waits(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerExternalTaskWaitScanRequest>,
) -> Result<Json<Vec<AgentWorkerExternalTaskWaitSnapshotWire>>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::list_agent_external_task_waits(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn wake_agent_external_task_wait(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerExternalTaskWaitWakeRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::wake_agent_external_task_wait(state.runtime(), &input).await?;
    Ok(Json(Value::Null))
}

pub(super) async fn claim_agent_inbox(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerInboxClaimRequest>,
) -> Result<Json<Option<AgentWorkerInboxSnapshotWire>>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::claim_agent_inbox(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn consume_agent_inbox(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerInboxConsumeRequest>,
) -> Result<Json<AgentWorkerInboxSnapshotWire>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::consume_agent_inbox(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn persist_agent_inbox(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerInboxPersistRequest>,
) -> Result<Json<AgentWorkerInboxPersistSnapshotWire>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::persist_agent_inbox(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn load_agent_inbox_message(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerInboxMessageRequest>,
) -> Result<Json<AgentWorkerMessageSnapshotWire>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::load_agent_inbox_message(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn apply_message_request(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerMessageRequest>,
) -> Result<Json<Option<AgentWorkerMessageSnapshotWire>>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::apply_message_request(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn worker_budget_snapshot(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerBudgetRequest>,
) -> Result<Json<AgentWorkerBudgetSnapshotWire>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::worker_budget_snapshot(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn fail_agent_worker_run(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerFailureRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::fail_agent_worker_run(state.runtime(), &input).await?;
    Ok(Json(Value::Null))
}

pub(super) async fn result_refs_for_agent_worker_run(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerResultRefsRequest>,
) -> Result<Json<AgentWorkerResultRefsSnapshotWire>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::result_refs_for_agent_worker_run(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn complete_agent_worker_run(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerCompletionRequest>,
) -> Result<Json<AgentWorkerCompletionSnapshotWire>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::complete_agent_worker_run(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn check_agent_worker_approval(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerApprovalCheckRequest>,
) -> Result<Json<AgentWorkerApprovalCheckSnapshotWire>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::check_agent_worker_approval(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn request_agent_worker_approval(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerApprovalRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::request_agent_worker_approval(state.runtime(), &input).await?;
    Ok(Json(Value::Null))
}

pub(super) async fn review_agent_delegation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerDelegationReviewRequest>,
) -> Result<Json<AgentWorkerDelegationReviewSnapshotWire>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::review_agent_delegation(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn inherit_agent_delegation_scope(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerDelegationScopeRequest>,
) -> Result<Json<AgentWorkerDelegationScopeSnapshotWire>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    agent_actions::inherit_agent_delegation_scope(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn ensure_agent_canonical_subagent_thread(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerDelegationThreadRequest>,
) -> Result<Json<AgentWorkerDelegationThreadSnapshotWire>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::ensure_agent_canonical_subagent_thread(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn append_agent_worker_update(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerEventRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::append_agent_worker_update(state.runtime(), &input).await?;
    Ok(Json(Value::Null))
}

pub(super) async fn create_agent_child_run(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerDelegationStartRequest>,
) -> Result<Json<AgentWorkerDelegationStartSnapshotWire>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::create_agent_child_run(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn command(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<AgentHostCommandPayload>>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    request
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    request
        .payload
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let user_id = agent_actions::authorize_agent_host_command(state.runtime(), &request).await?;
    let value =
        super::commands::dispatch_agent_host_command_value(state.runtime(), user_id, request)
            .await?;
    Ok(Json(value))
}

pub(super) async fn agent_package_execution_enabled(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<AgentWorkerPackageExecutionResponse>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    let enabled = agent_actions::agent_package_execution_enabled(state.runtime()).await?;
    Ok(Json(AgentWorkerPackageExecutionResponse { enabled }))
}

pub(super) async fn cancel_agent_generation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerCancelGenerationRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    agent_actions::cancel_agent_owned_generation_task(
        state.runtime(),
        input.user_id,
        &input.task_id,
        input.attempt,
    )
    .await?;
    Ok(Json(Value::Null))
}

pub(super) async fn list_agent_generation_cancel_candidates(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerGenerationCancelScanRequest>,
) -> Result<Json<Vec<AgentWorkerGenerationCancelSnapshotWire>>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::list_agent_generation_cancel_candidates(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn cancel_agent_workflow(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerCancelWorkflowRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    agent_actions::cancel_agent_owned_workflow(
        state.runtime(),
        input.workflow_run_id,
        &input.reason,
    )
    .await?;
    Ok(Json(Value::Null))
}

pub(super) async fn list_agent_workflow_cancel_candidates(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerWorkflowCancelScanRequest>,
) -> Result<Json<Vec<AgentWorkerWorkflowCancelSnapshotWire>>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::list_agent_workflow_cancel_candidates(state.runtime(), &input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn read_agent_wait(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerWaitRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    let value = agent_actions::read_external_task_wait(
        state.runtime(),
        AgentExternalTaskWaitSnapshot {
            run_id: input.run_id,
            owner_user_id: input.owner_user_id,
            scope: input.scope,
            execution_id: input.execution_id,
            tool_name: input.tool_name,
            arguments_json: input.arguments_json,
            duration_exhausted: input.duration_exhausted,
        },
        input.runtime_config_snapshot,
    )
    .await?;
    Ok(Json(value))
}

pub(super) async fn agent_llm_capabilities(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerLlmCapabilitiesRequest>,
) -> Result<Json<a4n_agent_runtime_contract::AgentLlmCapabilities>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::agent_llm_capabilities(
        state.runtime(),
        input.user_id,
        input.snapshot,
        input.config_id,
    )
    .await
    .map(Json)
    .map_err(Into::into)
}

pub(super) async fn agent_chat(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerChatRequest>,
) -> Result<Response, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let request = restore_model_request_wire(input.request)
        .map_err(|error| ApiError::bad_request(format!("Agent LLM request 无效: {error}")))?;
    let chunks = agent_actions::agent_chat_stream(
        state.runtime(),
        input.user_id,
        input.snapshot,
        input.config_id,
        request,
    )
    .await?;
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, no-transform"),
    );
    response_headers.insert("x-accel-buffering", HeaderValue::from_static("no"));
    Ok((
        response_headers,
        Sse::new(agent_chat_sse_stream(chunks)).keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(5))
                .text("keep-alive"),
        ),
    )
        .into_response())
}

pub(super) async fn prepare_agent_chat(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerChatRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let request = restore_model_request_wire(input.request)
        .map_err(|error| ApiError::bad_request(format!("Agent LLM request 无效: {error}")))?;
    let request = agent_actions::prepare_agent_chat_request(
        state.runtime(),
        input.user_id,
        input.snapshot,
        input.config_id,
        request,
    )
    .await?;
    Ok(Json(model_request_wire_value(&request)?))
}

pub(super) async fn project_agent_editor(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerEditorProjectionRequest>,
) -> Result<Json<VisualPlanProjection>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    agent_actions::project_agent_editor_draft(state.runtime(), &input.plan, &input.view)
        .map(Json)
        .map_err(Into::into)
}

fn agent_chat_sse_stream<S>(
    chunks: S,
) -> impl Stream<Item = Result<Event, Infallible>> + Send + 'static
where
    S: Stream<Item = Result<ChatChunk, AppError>> + Send + 'static,
{
    stream::unfold(
        (Box::pin(chunks), false),
        |(mut chunks, finished)| async move {
            if finished {
                return None;
            }
            match chunks.as_mut().next().await {
                Some(Ok(chunk)) => Some((Ok(chat_chunk_event(chunk)), (chunks, false))),
                Some(Err(error)) => Some((Ok(chat_error_event(error)), (chunks, true))),
                None => None,
            }
        },
    )
}

fn chat_chunk_event(chunk: ChatChunk) -> Event {
    Event::default()
        .event("chunk")
        .data(serde_json::to_string(&chunk).expect("ChatChunk must remain serializable"))
}

fn chat_error_event(error: AppError) -> Event {
    Event::default()
        .event("error")
        .data(serde_json::json!({ "message": error.to_string() }).to_string())
}

fn authorize_worker_request(state: &ServerState, headers: &HeaderMap) -> Result<(), ApiError> {
    let provided = headers
        .get(AGENT_WORKER_AUTH_HEADER)
        .and_then(|value| value.to_str().ok());
    if state.agent_worker_token_matches(provided) {
        Ok(())
    } else {
        Err(ApiError::unauthorized("Agent worker 身份验证失败"))
    }
}
