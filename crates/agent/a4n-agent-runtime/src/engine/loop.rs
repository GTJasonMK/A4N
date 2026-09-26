use super::*;
use std::sync::atomic::{AtomicI64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunProgress {
    Continue,
    Released,
}

/// 运行 Agent worker 生命周期。
///
/// Worker 持有 durable run driver 的生命周期；领取、恢复和状态机转移仍由
/// `AgentApplication` 负责，确保 Runtime 不复制控制平面规则。
pub async fn run_worker(
    state: AgentRuntimeState,
    cancel: CancellationToken,
    notify: Arc<Notify>,
    worker_id: String,
) {
    let maintenance_cancel = cancel.child_token();
    let maintenance_state = state.clone();
    let maintenance_token = maintenance_cancel.clone();
    let maintenance = tokio::spawn(async move {
        let mut tick = tokio::time::interval(WORKER_POLL_INTERVAL);
        tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = maintenance_token.cancelled() => break,
                _ = async {
                    tick.tick().await;
                    maintain_agent_worker(&maintenance_state).await;
                } => {}
            }
        }
    });
    let mut tasks = tokio::task::JoinSet::new();
    let mut interval = tokio::time::interval(WORKER_POLL_INTERVAL);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            result = tasks.join_next(), if !tasks.is_empty() => {
                if let Some(Err(error)) = result {
                    tracing::error!(%error, "Agent execution task terminated unexpectedly");
                }
                fill_available_runs(&state, &worker_id, &cancel, &mut tasks).await;
            }
            _ = notify.notified() => {
                fill_available_runs(&state, &worker_id, &cancel, &mut tasks).await;
            }
            _ = interval.tick() => {
                fill_available_runs(&state, &worker_id, &cancel, &mut tasks).await;
            }
        }
    }
    maintenance_cancel.cancel();
    state.shutdown().await;
    if tokio::time::timeout(Duration::from_secs(10), async {
        while let Some(result) = tasks.join_next().await {
            if let Err(error) = result {
                tracing::error!(%error, "Agent task shutdown failed");
            }
        }
    })
    .await
    .is_err()
    {
        tasks.shutdown().await;
        tracing::error!("Agent shutdown deadline elapsed; outstanding leases require recovery");
    }
    if let Err(error) = maintenance.await {
        tracing::error!(%error, "Agent maintenance task shutdown failed");
    }
}

async fn fill_available_runs(
    state: &AgentRuntimeState,
    worker_id: &str,
    cancel: &CancellationToken,
    tasks: &mut tokio::task::JoinSet<()>,
) {
    while tasks.len() < WORKER_MAX_CONCURRENT_RUNS && !cancel.is_cancelled() {
        let request = match tokio::select! {
            _ = cancel.cancelled() => break,
            result = claim_agent_worker_request(state, worker_id) => result,
        } {
            Ok(Some(request)) => request,
            Ok(None) => break,
            Err(error) => {
                tracing::error!(error = %error, "Agent worker failed to claim run");
                break;
            }
        };
        let claimed = match agent_worker_run_from_wire(request.payload.worker_run.clone()) {
            Ok(claimed) => claimed,
            Err(error) => {
                tracing::error!(error = %error, "Agent worker claim returned an invalid run");
                break;
            }
        };
        let run_id = claimed.run.id;
        let state = state.clone();
        tasks.spawn(async move {
            if let Err(error) = execute_claimed_worker_run(&state, claimed, request).await {
                tracing::error!(run_id, error = %error, "Agent worker run failed");
            }
        });
    }
}

/// Runs the durable maintenance scans that must continue when Agent execution
/// lives in an external process rather than inside Control.
pub async fn maintain_agent_worker(state: &AgentRuntimeState) {
    let approval_expiry = AgentWorkerApprovalExpiryRequest { limit: 32 };
    if let Err(error) = state
        .host()
        .expire_pending_approvals(&approval_expiry)
        .await
    {
        tracing::warn!(error = %error, "Agent approval expiry scan failed");
    }
    let editor_wait_expiry = AgentWorkerEditorWaitExpiryRequest { limit: 32 };
    if let Err(error) = state.host().expire_editor_waits(&editor_wait_expiry).await {
        tracing::warn!(error = %error, "Agent editor wait expiry scan failed");
    }
    if let Err(error) = task_wait::reconcile(state).await {
        tracing::warn!(error = %error, "Agent task wait reconciliation failed");
    }
    let recovery = AgentWorkerExpiredRunRecoveryRequest {
        limit: 16,
        error_message: Some("Agent worker lease expired".into()),
    };
    if let Err(error) = state.host().recover_expired_runs(&recovery).await {
        tracing::warn!(error = %error, "Agent expired-run recovery failed");
    }
}

pub async fn claim_agent_worker_request(
    state: &AgentRuntimeState,
    worker_id: &str,
) -> Result<Option<WorkerEnvelope<AgentRunPayload>>, AppError> {
    if worker_id.trim().is_empty() {
        return Err(AppError::InvalidArgument("Agent worker_id 不能为空".into()));
    }
    let _admission = state.lock_agent_admission().await;
    if !state.host().package_execution_enabled().await? {
        return Ok(None);
    }
    let request = AgentWorkerClaimRequest {
        worker_id: worker_id.to_string(),
    };
    request.validate().map_err(|error| AppError::InvalidState {
        current: format!("Agent worker claim request 无效: {error}"),
        operation: "领取 Agent worker run".into(),
    })?;
    let Some(mut request) = state.host().claim_worker(&request).await? else {
        return Ok(None);
    };
    if request.payload.runtime_config_snapshot.is_none() {
        let mut snapshot = state.agent_runtime_snapshot(request.payload.run_id).await;
        if snapshot.is_none()
            && let Some(parent_id) = request.payload.worker_run.run.parent_run_id
        {
            snapshot = state.agent_runtime_snapshot(parent_id).await;
        }
        request.payload.runtime_config_snapshot = snapshot.map(serde_json::to_value).transpose()?;
    }
    Ok(Some(request))
}

pub async fn execute_claimed_worker_run(
    state: &AgentRuntimeState,
    claimed: AgentWorkerRun,
    request: WorkerEnvelope<AgentRunPayload>,
) -> Result<(), AppError> {
    validate_claimed_worker_request(&claimed, &request)?;
    if let Some(value) = request.payload.runtime_config_snapshot.clone() {
        let snapshot = serde_json::from_value(value).map_err(|error| AppError::InvalidState {
            current: format!("Agent worker request runtime snapshot 无效: {error}"),
            operation: "加载 Agent worker runtime snapshot".into(),
        })?;
        state
            .store_agent_runtime_snapshot(claimed.run.id, claimed.run.parent_run_id, snapshot)
            .await;
    }
    process_claimed_run_request(state, claimed, request).await
}

async fn process_claimed_run_request(
    state: &AgentRuntimeState,
    claimed: AgentWorkerRun,
    request: WorkerEnvelope<AgentRunPayload>,
) -> Result<(), AppError> {
    let failure_application = state.worker_failure_application(&claimed);
    let run_cancel = state
        .register_agent_run_cancellation(claimed.run.id, claimed.run.parent_run_id)
        .await;
    let worker_cancel = state.cancel_token();
    let renew_cancel = worker_cancel.child_token();
    let renew_token = renew_cancel.clone();
    let lost_lease = run_cancel.clone();
    let renew_claim = claimed.clone();
    let renew_state = state.clone();
    let renew_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(WORKER_RENEW_INTERVAL);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = renew_token.cancelled() => break,
                _ = interval.tick() => {
                    if let Err(error) = renew_state.renew_agent_worker_lease(&renew_claim).await {
                        tracing::warn!(run_id = renew_claim.run.id, error = %error, "Agent worker lease renewal failed");
                        lost_lease.cancel();
                        break;
                    }
                }
            }
        }
    });

    // Keep the loop state out of every outer worker future's debug poll stack.
    let outcome: Result<_, AppError> = async {
        let budget: AgentBudget = serde_json::from_value(claimed.run.budget_json.clone())?;
        validate_agent_budget(&budget)?;
        Ok(tokio::select! {
            biased;
            _ = worker_cancel.cancelled() => Some(Err(AppError::Cancelled)),
            _ = run_cancel.cancelled() => Some(Ok(())),
            elapsed = wait_for_duration_budget(state, &claimed) => match elapsed {
                Ok(()) => None,
                Err(error) => Some(Err(error)),
            },
            result = Box::pin(process_claimed_run_inner(
                state,
                &claimed,
                &run_cancel,
                &request,
            )) => Some(result),
        })
    }
    .await;
    // Drop the in-flight call before persisting the wait; a continuously active
    // stream must not extend the run's duration budget or keep its HTTP body alive.
    let result = match outcome {
        Ok(Some(result)) => result,
        Ok(None) => suspend_agent_at_duration_limit(state, &claimed, &request).await,
        Err(error) => Err(error),
    };
    renew_cancel.cancel();
    renew_handle.abort();
    if let Err(error) = renew_handle.await
        && !error.is_cancelled()
    {
        tracing::error!(run_id = claimed.run.id, %error, "Agent lease task failed");
    }
    state
        .unregister_agent_run_cancellation(claimed.run.id)
        .await;
    let mut terminal = false;
    if worker_cancel.is_cancelled() {
        match state
            .mark_agent_recovery_required(
                &claimed,
                "Agent worker 已关闭，保留执行记录供恢复/对账".into(),
            )
            .await
        {
            Ok(()) => terminal = true,
            Err(error) => {
                tracing::warn!(run_id = claimed.run.id, %error, "Agent shutdown could not persist recovery state")
            }
        }
    }
    if let Err(error) = &result
        && !run_cancel.is_cancelled()
        && !worker_cancel.is_cancelled()
    {
        match failure_application
            .fail_child_run(&claimed, error.to_string())
            .await
        {
            Ok(_) => terminal = true,
            Err(failure) => {
                tracing::warn!(run_id = claimed.run.id, error = %failure, "Agent worker could not persist failure");
            }
        }
    }
    if terminal {
        let _ = state.take_agent_runtime_snapshot(claimed.run.id).await;
    }
    result
}

async fn wait_for_duration_budget(
    state: &AgentRuntimeState,
    claimed: &AgentWorkerRun,
) -> Result<(), AppError> {
    loop {
        let budget = state
            .worker_budget_application(claimed)
            .worker_budget_snapshot(claimed.run.id)
            .await?;
        if budget.max_duration_seconds == Some(0) {
            return Ok(());
        }
        let Some(deadline) = budget
            .usage
            .get("duration_deadline_unix_ms")
            .filter(|value| !value.is_null())
        else {
            return std::future::pending().await;
        };
        let deadline = deadline.as_i64().ok_or_else(|| AppError::InvalidState {
            current: "Agent 预算截止时间无效".into(),
            operation: "执行时长预算".into(),
        })?;
        let millis = deadline
            .saturating_sub(chrono::Utc::now().timestamp_millis())
            .max(1) as u64;
        let wake = tokio::time::Instant::now()
            .checked_add(Duration::from_millis(millis))
            .ok_or_else(|| AppError::InvalidArgument("Agent 时长预算超出时钟范围".into()))?;
        tokio::time::sleep_until(wake).await;
        // An explicit parent extension may have changed this deadline while
        // the request was streaming. Recheck without dropping a valid stream.
    }
}

async fn suspend_agent_at_duration_limit(
    state: &AgentRuntimeState,
    claimed: &AgentWorkerRun,
    request: &WorkerEnvelope<AgentRunPayload>,
) -> Result<(), AppError> {
    let context = state.load_agent_run_context(request).await?;
    for execution in context
        .executions
        .iter()
        .filter(|execution| execution.status == "running")
    {
        if let Some(client_message_id) = stream_client_message_id(claimed.run.id, execution) {
            state
                .worker_message_application(claimed)
                .interrupt_assistant_streaming_message(
                    claimed,
                    client_message_id,
                    Some("Agent 已达到时长预算，等待调整预算后继续".into()),
                )
                .await?;
        }
    }
    state.wait_agent_worker_run(claimed, "budget").await
}

async fn process_claimed_run_inner(
    state: &AgentRuntimeState,
    claimed: &AgentWorkerRun,
    cancel: &CancellationToken,
    request: &WorkerEnvelope<AgentRunPayload>,
) -> Result<(), AppError> {
    tracing::debug!(
        run_id = claimed.run.id,
        request_id = %request.request_id,
        operation_id = %request.operation_id,
        "Agent worker request contract validated"
    );
    let execution_application = state.worker_execution_application(claimed);
    let mut context = state.load_agent_run_context(request).await?;
    let snapshot = state.agent_runtime_snapshot(claimed.run.id).await;
    if context.runtime_snapshot_required && snapshot.is_none() {
        state
            .mark_agent_recovery_required(
                claimed,
                "Agent runtime config snapshot 已丢失，无法安全恢复模型或工具调用".into(),
            )
            .await?;
        return Ok(());
    }
    state.mark_agent_turn_active(claimed).await?;
    // A model response can be durable before its assistant message. Materialize
    // that decision first, including when a token limit suspended the Run.
    let recovered_final = recover_model_messages(
        &state.worker_message_application(claimed),
        claimed,
        &mut context,
    )
    .await?;
    let mut messages = chat_messages(&context)?;
    let mut loaded_message_ids: HashSet<i64> =
        context.messages.iter().map(|message| message.id).collect();
    let latest_message_id = AtomicI64::new(loaded_message_ids.iter().copied().max().unwrap_or(0));
    let mut pinned_user_contents = context
        .messages
        .iter()
        .filter(|message| message.turn_id == Some(context.turn.id) && message.role == "user")
        .map(|message| message.content.clone())
        .collect::<Vec<_>>();
    append_inbox_messages(
        state,
        claimed,
        &mut messages,
        &mut loaded_message_ids,
        &latest_message_id,
        &mut pinned_user_contents,
    )
    .await?;
    if messages.is_empty() {
        return Err(AppError::InvalidState {
            current: format!("Agent run {} 没有可发送的消息", claimed.run.id),
            operation: "执行 Agent run".into(),
        });
    }

    let budget: AgentBudget =
        serde_json::from_value(context.budget.clone()).map_err(|error| AppError::InvalidState {
            current: format!("Agent run {} budget_json 无效: {error}", claimed.run.id),
            operation: "执行 Agent run".into(),
        })?;
    validate_agent_budget(&budget)?;
    let mut usage = usage_object(&context.usage)?;
    if usage.get("phase").and_then(Value::as_str).is_none() {
        usage["phase"] = json!(a4n_api_contract::agent::AGENT_PHASE_DISCOVER);
        usage["phase_revision"] = json!(0);
    }
    let config_id = context
        .turn
        .model_override_json
        .as_ref()
        .and_then(|value| value.get("model_config_id"))
        .and_then(Value::as_i64)
        .or(context.default_model_config_id);
    // Keep discovered tools in the worker only. They are re-established from
    // the durable search/describe observation after a restart, while the
    // manifest remains the single source of executable schemas and policy.
    let mut discovered_tools = Vec::new();
    for message in &context.messages {
        if let Some(payload) = &message.payload_json
            && payload.get("tool").and_then(Value::as_str) == Some("describe_agent_tool")
        {
            extend_discovered_tools(
                &mut discovered_tools,
                discovered_tool_names("describe_agent_tool", &payload.get("result").cloned())?,
                &context.scope,
            )?;
        }
    }
    let discovery_cache = Arc::new(Mutex::new(successful_discovery_cache(&context.executions)));
    let retry_counts = Arc::new(Mutex::new(observation::restored_retry_counts(
        &context.executions,
    )?));
    let executor = RuntimeAgentToolExecutor::new(state.clone(), request.clone());

    if let Some((content, source_execution_id)) = recovered_final {
        persist_usage(state, claimed, &usage).await?;
        let source = context
            .executions
            .iter()
            .find(|execution| execution.id == source_execution_id)
            .ok_or_else(|| AppError::InvalidState {
                current: "恢复的完成决定缺少模型执行".into(),
                operation: "恢复 Agent".into(),
            })?;
        if complete_run(state, claimed, content, source, &mut messages).await?
            == RunProgress::Released
        {
            return Ok(());
        }
    }

    // Approval/child completion requeues this Run. Resume its original calls
    // before asking the model to make another decision with an incomplete batch.
    let current_messages = context
        .messages
        .iter()
        .filter(|message| message.run_id == Some(claimed.run.id))
        .map(chat_message_from_model)
        .collect::<Result<Vec<_>, _>>()?;
    for calls in history::pending_tool_batches(&current_messages)? {
        if execute_tool_calls(
            ToolBatchContext {
                state,
                claimed,
                context: &context,
                executor: &executor,
                budget: &budget,
                discovery_cache: &discovery_cache,
                retry_counts: &retry_counts,
                discovered_tools: &mut discovered_tools,
                messages: &mut messages,
                latest_message_id: &latest_message_id,
                usage: &mut usage,
            },
            calls,
        )
        .await?
            == RunProgress::Released
        {
            return Ok(());
        }
    }

    loop {
        if cancel.is_cancelled() {
            return Ok(());
        }
        append_inbox_messages(
            state,
            claimed,
            &mut messages,
            &mut loaded_message_ids,
            &latest_message_id,
            &mut pinned_user_contents,
        )
        .await?;
        context = state.load_agent_run_context(request).await?;
        *retry_counts.lock().await = observation::restored_retry_counts(&context.executions)?;
        messages = chat_messages(&context)?;
        loaded_message_ids.extend(context.messages.iter().map(|message| message.id));
        pinned_user_contents = context
            .messages
            .iter()
            .filter(|message| message.turn_id == Some(context.turn.id) && message.role == "user")
            .map(|message| message.content.clone())
            .collect();
        let model_execution_key = model::next_model_execution_key(&context.executions)?;
        let existing_model_execution = state
            .worker_execution_by_key(claimed, &model_execution_key)
            .await?;
        let request = if let Some(existing) = existing_model_execution.as_ref() {
            model_execution_request(existing)?
        } else {
            let tool_window =
                available_runtime_tools(&context.profile_id, &context.scope, &discovered_tools)?;
            usage["tool_window"] = json!({
                "schema_limit": MAX_AGENT_TOOL_SCHEMAS,
                "dropped_optional": tool_window.dropped_optional,
            });
            let tools = tool_window.schemas;
            let skills = if usage.get("failure_class").and_then(Value::as_str)
                == Some(a4n_agent_runtime_contract::AGENT_FAILURE_PROVIDER_UNAVAILABLE)
            {
                AgentSkillSearchResult {
                    skills: Vec::new(),
                    total: 0,
                }
            } else {
                state
                    .search_agent_skills(claimed, SearchAgentSkillsInput::default())
                    .await?
            };
            let snapshot = state.agent_runtime_snapshot(claimed.run.id).await;
            let llm_capabilities = match state
                .host()
                .llm_capabilities(context.owner_user_id, snapshot.as_ref(), config_id)
                .await
            {
                Ok(capabilities) => capabilities,
                Err(error) => {
                    record_provider_failure(state, claimed, &mut usage, &error).await?;
                    return Err(error);
                }
            };
            let capabilities = llm_capabilities.model;
            let requested_output_tokens = validated_model_output_tokens(&capabilities)?;
            let protocol = llm_capabilities.agent_protocol;
            let base_messages = with_agent_system_prompt(
                history::pair_tool_messages(&messages)?,
                &context,
                &skills,
                &tools,
                &protocol,
                usage
                    .get("phase")
                    .and_then(Value::as_str)
                    .unwrap_or(a4n_api_contract::agent::AGENT_PHASE_DISCOVER),
            );
            let mut projection = history::fit_context_projection(
                base_messages,
                &tools,
                capabilities.max_context_tokens,
                requested_output_tokens,
                &pinned_user_contents
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            )?;
            let latest_message_sequence = latest_message_id.load(Ordering::Relaxed);
            while projection.omitted_history
                || projection.input_tokens.saturating_mul(100)
                    >= projection.capacity.saturating_mul(80)
            {
                let source_budget = capabilities.max_context_tokens.saturating_mul(65) / 100;
                let Some((source, covered)) = history::summary_source(&context, source_budget)?
                else {
                    if projection.omitted_history {
                        return Err(AppError::InvalidArgument(
                            "完整历史批次无法装入摘要请求；原始记录已保留，请使用更大上下文的模型"
                                .into(),
                        ));
                    }
                    break;
                };
                let outcome = compact_context(
                    state,
                    claimed,
                    &mut context,
                    covered,
                    source,
                    config_id,
                    capabilities.max_context_tokens,
                    requested_output_tokens,
                )
                .await?;
                if outcome == RunProgress::Released {
                    return Ok(());
                }
                messages = chat_messages(&context)?;
                projection = history::fit_context_projection(
                    with_agent_system_prompt(
                        history::pair_tool_messages(&messages)?,
                        &context,
                        &skills,
                        &tools,
                        &protocol,
                        usage
                            .get("phase")
                            .and_then(Value::as_str)
                            .unwrap_or(a4n_api_contract::agent::AGENT_PHASE_DISCOVER),
                    ),
                    &tools,
                    capabilities.max_context_tokens,
                    requested_output_tokens,
                    &pinned_user_contents
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>(),
                )?;
            }
            let output_tokens = projection.output_tokens;
            let request_messages = projection.messages;
            usage["context_input_tokens"] = json!(projection.input_tokens);
            usage["context_capacity_tokens"] = json!(projection.capacity);
            persist_usage(state, claimed, &usage).await?;
            let mut request = ChatRequest {
                messages: request_messages,
                vision_images: Vec::new(),
                temperature: Some(0.2),
                max_tokens: Some(output_tokens),
                response_format: None,
                stream_include_usage: true,
                cache_scope_key: None,
                cache_breakpoints: Vec::new(),
                cache_mode: None,
                trace_context: Some(LlmTraceContext {
                    project_id: context.scope.project_id,
                    root_project_id: None,
                    workflow: "agent".into(),
                    step_key: format!("run:{}:{}", claimed.run.id, model_execution_key),
                    generation_run_id: None,
                    workflow_run_id: None,
                    call_key: Some(model_execution_key.clone()),
                    input_fingerprint_sha256: None,
                    context_plan_json: Some(json!({
                        "owner": "agent-runtime",
                        "history_message_id": latest_message_sequence,
                        "summary_message_id": context.thread.context_summary.as_ref().and_then(|value| value.get("covered_through_sequence")),
                        "input_tokens": projection.input_tokens,
                        "capacity_tokens": projection.capacity,
                    })),
                    context_pack_refs_json: None,
                }),
                // Agent history is already projected as complete immutable
                // batches. The single-user manifest renderer must not rewrite
                // an old user instruction or insert a second context owner.
                context_manifest_json: None,
                tools: None,
                tool_choice: None,
                stop_sequences: None,
            };
            configure_agent_request(&mut request, &protocol, tools);
            if let Some(remaining) = state
                .worker_budget_application(claimed)
                .worker_budget_snapshot(claimed.run.id)
                .await?
                .max_tokens
            {
                let reserved = a4n_domain::agent::agent_model_token_reservation(
                    &serde_json::to_value(&request)?,
                );
                let input_tokens = reserved.saturating_sub(output_tokens as u64);
                if remaining > input_tokens {
                    request.max_tokens =
                        Some(output_tokens.min((remaining - input_tokens) as usize));
                }
            }
            let snapshot = state.agent_runtime_snapshot(claimed.run.id).await;
            request = state
                .host()
                .prepare_chat_request(context.owner_user_id, snapshot.as_ref(), config_id, request)
                .await?;
            request
        };
        let model_execution = if let Some(existing) = existing_model_execution {
            existing
        } else {
            let Some(model_execution) = execution_application
                .create_prepared_execution(WorkerExecutionInput {
                    run_id: claimed.run.id,
                    lease_token: claimed.lease_token.clone(),
                    execution_key: model_execution_key,
                    execution_kind: "model_call".into(),
                    tool_name: None,
                    effect: Some("read".into()),
                    // Model requests have no user-visible side effect; use the
                    // canonical replay policy wire value so the durable execution
                    // can be projected and resumed after a worker restart.
                    replay_policy: Some("safe".into()),
                    arguments_json: Some(request.snapshot_value()?),
                    resource_preconditions_json: Some(
                        json!({"input_message_id": loaded_message_ids.iter().max()}),
                    ),
                })
                .await?
            else {
                state.wait_agent_worker_run(claimed, "budget").await?;
                return Ok(());
            };
            model_execution
        };
        let request = model_execution_request(&model_execution)?;
        let replayed_model_execution = model_execution.status == "succeeded";
        let request_protocol =
            agent_protocol_from_request_snapshot(model_execution_snapshot(&model_execution)?)
                .to_string();
        let mut response: ChatResponse = if replayed_model_execution {
            model_response(&model_execution)?
        } else {
            execution_application
                .update_worker_execution(WorkerExecutionUpdate {
                    run_id: claimed.run.id,
                    lease_token: claimed.lease_token.clone(),
                    execution_id: model_execution.id,
                    status: "running".into(),
                    result_json: None,
                    result_summary: None,
                    result_refs_json: None,
                    error_message: None,
                })
                .await?;
            let stream_client_message_id =
                stream_client_message_id(claimed.run.id, &model_execution).ok_or_else(|| {
                    AppError::InvalidState {
                        current: "主模型执行缺少流身份".into(),
                        operation: "启动模型流".into(),
                    }
                })?;
            let snapshot = state.agent_runtime_snapshot(claimed.run.id).await;
            let call = state.host().chat_stream(
                context.owner_user_id,
                snapshot.as_ref(),
                config_id,
                request,
            );
            let stream = tokio::select! {
                _ = cancel.cancelled() => return Ok(()),
                stream = call => match stream {
                    Ok(stream) => stream,
                    Err(error) => {
                        record_provider_failure(
                            state,
                            claimed,
                            &mut usage,
                            &error,
                        )
                        .await?;
                        return Err(error);
                    }
                },
            };
            match consume_model_stream(
                stream,
                &state.worker_message_application(claimed),
                claimed,
                &stream_client_message_id,
                cancel,
            )
            .await
            {
                Ok(response) => response,
                Err(error) => {
                    record_provider_failure(state, claimed, &mut usage, &error).await?;
                    return Err(error);
                }
            }
        };
        let decision = decode_agent_response(&response, &request_protocol, model_execution.id).map(
            |mut decision| {
                if !replayed_model_execution {
                    model::namespace_tool_calls(&mut response, &mut decision, model_execution.id);
                }
                decision
            },
        );
        if !replayed_model_execution {
            execution_application
                .update_worker_execution(WorkerExecutionUpdate {
                    run_id: claimed.run.id,
                    lease_token: claimed.lease_token.clone(),
                    execution_id: model_execution.id,
                    status: if decision.is_ok() {
                        "succeeded"
                    } else {
                        "failed"
                    }
                    .into(),
                    result_json: Some(serde_json::to_value(&response)?),
                    result_summary: Some(
                        if decision.is_ok() {
                            "Agent model call validated"
                        } else {
                            "Agent model call validation failed"
                        }
                        .into(),
                    ),
                    result_refs_json: None,
                    error_message: decision.as_ref().err().map(ToString::to_string),
                })
                .await?;
        }
        let decision = decision?;
        let (assistant_content, calls, is_final) = match decision {
            AgentModelDecision::Final { reply } => (reply, Vec::new(), true),
            AgentModelDecision::Think { content } => (content, Vec::new(), false),
            AgentModelDecision::ToolCalls { content, calls } => (content, calls, false),
        };
        if is_final {
            usage["phase"] = json!(a4n_api_contract::agent::AGENT_PHASE_FINAL);
            persist_usage(state, claimed, &usage).await?;
            if complete_run(
                state,
                claimed,
                assistant_content,
                &model_execution,
                &mut messages,
            )
            .await?
                == RunProgress::Released
            {
                return Ok(());
            }
            usage["phase"] = json!(a4n_api_contract::agent::AGENT_PHASE_VERIFY);
            continue;
        }
        let mut assistant = ChatMessage::assistant(assistant_content.clone());
        assistant.tool_calls = (!calls.is_empty()).then_some(calls.clone());
        // Persist the assistant decision before dispatching side effects so a
        // resumed run can reconstruct the exact tool-call context.
        let tool_calls_payload = serde_json::to_value(&calls)?;
        let message_application = state.worker_message_application(claimed);
        let finalized = if let Some(client_message_id) =
            stream_client_message_id(claimed.run.id, &model_execution)
        {
            message_application
                .complete_assistant_streaming_message(
                    claimed,
                    client_message_id,
                    assistant_content.clone(),
                    tool_calls_payload.clone(),
                )
                .await?
        } else {
            None
        };
        let persisted_assistant = if let Some(message) = finalized {
            message
        } else {
            message_application
                .append_assistant_tool_message(
                    claimed,
                    assistant_content,
                    tool_calls_payload,
                    format!("agent-decision:{}", model_execution.id),
                )
                .await?
        };
        history::advance_message_watermark(&latest_message_id, persisted_assistant.id);
        messages.push(assistant);
        if execute_tool_calls(
            ToolBatchContext {
                state,
                claimed,
                context: &context,
                executor: &executor,
                budget: &budget,
                discovery_cache: &discovery_cache,
                retry_counts: &retry_counts,
                discovered_tools: &mut discovered_tools,
                messages: &mut messages,
                latest_message_id: &latest_message_id,
                usage: &mut usage,
            },
            calls,
        )
        .await?
            == RunProgress::Released
        {
            return Ok(());
        }
        state
            .update_agent_worker_usage(claimed, usage.clone())
            .await?;
    }
}
