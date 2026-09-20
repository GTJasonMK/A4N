use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunProgress {
    Continue,
    Released,
}

/// 运行 Agent worker 生命周期。
///
/// Worker 持有 durable run driver 的生命周期；领取、恢复和状态机转移仍由
/// `AgentApplication` 负责，确保 Runtime 不复制控制平面规则。
pub async fn run_worker(state: AgentRuntimeState, cancel: CancellationToken, notify: Arc<Notify>) {
    let mut interval = tokio::time::interval(WORKER_POLL_INTERVAL);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    // interval 的第一次 tick 立即就绪，启动时主动扫描一次，避免错过启动前已排队的 Run。
    interval.tick().await;
    drain_runnable_runs(&state).await;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = notify.notified() => {
                drain_runnable_runs(&state).await;
            }
            _ = interval.tick() => {
                drain_runnable_runs(&state).await;
            }
        }
    }
}

async fn drain_runnable_runs(state: &AgentRuntimeState) {
    let worker_id = format!("a4n-agent:{}", std::process::id());
    maintain_agent_worker(state).await;

    for _ in 0..WORKER_MAX_CLAIMS_PER_WAKEUP {
        let request = match claim_agent_worker_request(state, &worker_id).await {
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
        if let Err(error) = execute_claimed_worker_run(state, claimed, request).await {
            tracing::error!(run_id, error = %error, "Agent worker run failed");
        }
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
) -> Result<Option<WorkerRequest<AgentRunPayload>>, AppError> {
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
    if request.payload.runtime_config_snapshot.is_none()
        && let Some(snapshot) = state.agent_runtime_snapshot(request.payload.run_id).await
    {
        request.payload.runtime_config_snapshot = Some(serde_json::to_value(snapshot)?);
    }
    Ok(Some(request))
}

pub async fn execute_claimed_worker_run(
    state: &AgentRuntimeState,
    claimed: AgentWorkerRun,
    request: WorkerRequest<AgentRunPayload>,
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
    request: WorkerRequest<AgentRunPayload>,
) -> Result<(), AppError> {
    let failure_application = state.worker_failure_application(&claimed);
    let run_cancel = state
        .register_agent_run_cancellation(claimed.run.id, claimed.run.parent_run_id)
        .await;
    let renew_cancel = CancellationToken::new();
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
        let deadline = agent_duration_deadline(
            &budget,
            claimed.run.started_at.unwrap_or(claimed.run.created_at),
        )?;
        Ok(tokio::select! {
            biased;
            _ = async {
                match deadline {
                    Some(deadline) => tokio::time::sleep_until(deadline).await,
                    None => std::future::pending::<()>().await,
                }
            } => None,
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
    state
        .unregister_agent_run_cancellation(claimed.run.id)
        .await;
    let mut terminal = false;
    if let Err(error) = &result
        && !run_cancel.is_cancelled()
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
    if result.is_ok()
        && let Ok(is_terminal) = state.agent_worker_run_is_terminal(&claimed).await
    {
        terminal = is_terminal;
    }
    if terminal {
        let _ = state.take_agent_runtime_snapshot(claimed.run.id).await;
    }
    result
}

fn agent_duration_deadline(
    budget: &AgentBudget,
    started_at: chrono::NaiveDateTime,
) -> Result<Option<tokio::time::Instant>, AppError> {
    let Some(seconds) = budget.max_duration_seconds else {
        return Ok(None);
    };
    let elapsed = chrono::Utc::now()
        .naive_utc()
        .signed_duration_since(started_at)
        .to_std()
        .unwrap_or_default();
    tokio::time::Instant::now()
        .checked_add(Duration::from_secs(seconds).saturating_sub(elapsed))
        .map(Some)
        .ok_or_else(|| AppError::InvalidArgument("Agent max_duration_seconds 超出时钟范围".into()))
}

async fn suspend_agent_at_duration_limit(
    state: &AgentRuntimeState,
    claimed: &AgentWorkerRun,
    request: &WorkerRequest<AgentRunPayload>,
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
    request: &WorkerRequest<AgentRunPayload>,
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
    let mut loaded_message_ids = context.messages.iter().map(|message| message.id).collect();
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
    let mut steps = json_u32(&usage, "steps_used")?;
    let mut model_calls = json_u32(&usage, "model_calls_used")?;
    let mut tool_calls = json_u32(&usage, "tool_calls_used")?;
    let mut tokens = json_u64(&usage, "tokens_used")?;
    if usage.get("phase").and_then(Value::as_str).is_none() {
        usage["phase"] = json!(a4n_api_contract::agent::AGENT_PHASE_DISCOVER);
        usage["phase_revision"] = json!(0);
    }
    let model_executions = context
        .executions
        .iter()
        .filter(|execution| execution.execution_kind == "model_call")
        .collect::<Vec<_>>();
    model_calls = model_calls.max(u32::try_from(model_executions.len()).map_err(|_| {
        AppError::InvalidState {
            current: "Agent 模型执行计数超出 u32".into(),
            operation: "恢复模型计数".into(),
        }
    })?);
    steps = steps.max(model_calls);
    let mut recorded_tokens = 0_u64;
    for execution in model_executions {
        if execution.status == "succeeded" {
            recorded_tokens = recorded_tokens.saturating_add(execution_total_tokens(execution)?);
        }
    }
    tokens = tokens.max(recorded_tokens);
    let started_at = context.run.started_at.unwrap_or(context.run.created_at);
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
    for execution in &context.executions {
        if execution.status == "succeeded"
            && let Some(tool_name) = execution.tool_name.as_deref()
        {
            extend_discovered_tools(
                &mut discovered_tools,
                discovered_tool_names(tool_name, &execution.result_json)?,
                &context.scope,
            )?;
        }
    }
    let mut prepared_calls = context
        .executions
        .iter()
        .filter_map(|execution| execution.execution_key.strip_prefix("tool-call:"))
        .map(str::to_string)
        .collect::<HashSet<_>>();
    let discovery_cache = Arc::new(Mutex::new(successful_discovery_cache(&context.executions)));
    let retry_counts = Arc::new(Mutex::new(observation::restored_retry_counts(
        &context.executions,
    )?));
    tool_calls = tool_calls.max(u32::try_from(prepared_calls.len()).map_err(|_| {
        AppError::InvalidState {
            current: "Agent 工具执行计数超出 u32".into(),
            operation: "恢复 Agent 工具执行".into(),
        }
    })?);
    let executor = RuntimeAgentToolExecutor::new(state.clone(), request.clone());

    if let Some((content, input_message_id)) = recovered_final {
        persist_usage(
            state,
            claimed,
            &usage,
            steps,
            model_calls,
            tool_calls,
            tokens,
        )
        .await?;
        complete_run(state, claimed, content, input_message_id).await?;
        return Ok(());
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
                prepared_calls: &mut prepared_calls,
                discovery_cache: &discovery_cache,
                retry_counts: &retry_counts,
                discovered_tools: &mut discovered_tools,
                messages: &mut messages,
                usage: &mut usage,
                tool_calls: &mut tool_calls,
                steps,
                model_calls,
                tokens,
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
            &mut pinned_user_contents,
        )
        .await?;
        if steps >= budget.max_steps.unwrap_or(WORKER_MAX_STEPS)
            || model_calls >= budget.max_model_calls.unwrap_or(WORKER_MAX_STEPS)
            || action_budget_exhausted(&budget, tokens, started_at)
        {
            persist_usage(
                state,
                claimed,
                &usage,
                steps,
                model_calls,
                tool_calls,
                tokens,
            )
            .await?;
            state.wait_agent_worker_run(claimed, "budget").await?;
            return Ok(());
        }
        steps = steps.saturating_add(1);
        model_calls = model_calls.saturating_add(1);
        usage["steps_used"] = json!(steps);
        usage["model_calls_used"] = json!(model_calls);
        let model_execution_key = format!("model-call:{model_calls}");
        let existing_model_execution = state
            .worker_execution_by_key(claimed, &model_execution_key)
            .await?;
        let request = if let Some(existing) = existing_model_execution.as_ref() {
            model_execution_request(existing)?
        } else {
            let tool_window = available_runtime_tools(
                &context.profile_id,
                &context.scope,
                usage
                    .get("phase")
                    .and_then(Value::as_str)
                    .unwrap_or(a4n_api_contract::agent::AGENT_PHASE_DISCOVER),
                &discovered_tools,
                &context.executions,
            )?;
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
                    record_provider_failure(
                        state,
                        claimed,
                        &mut usage,
                        UsageCounters {
                            steps,
                            model_calls,
                            tool_calls,
                            tokens,
                        },
                        &error,
                    )
                    .await?;
                    return Err(error);
                }
            };
            let capabilities = llm_capabilities.model;
            let requested_output_tokens = validated_model_output_tokens(&capabilities)?;
            let protocol = match snapshot.as_ref() {
                Some(snapshot) => agent_protocol_for_snapshot(Some(snapshot), config_id)?,
                None => llm_capabilities.agent_protocol,
            };
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
            let projection = history::fit_context_projection(
                base_messages,
                &tools,
                capabilities.max_context_tokens,
                requested_output_tokens,
                &pinned_user_contents
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            )?;
            let mut context_input_tokens = projection.input_tokens;
            let mut context_capacity_tokens = projection.capacity;
            let latest_message_sequence = loaded_message_ids.iter().copied().max().unwrap_or(0);
            let summary_covered_sequence = context
                .thread
                .context_summary
                .as_ref()
                .and_then(|summary| summary.get("covered_through_sequence"))
                .and_then(Value::as_i64)
                .unwrap_or(-1);
            let can_spend_summary_call = budget.max_steps.is_none_or(|limit| model_calls < limit)
                && budget
                    .max_model_calls
                    .is_none_or(|limit| model_calls < limit);
            let mut output_tokens = projection.output_tokens;
            let mut request_messages = projection.messages;
            if projection.input_tokens.saturating_mul(100) >= projection.capacity.saturating_mul(80)
                && !projection.omitted.is_empty()
                && latest_message_sequence > summary_covered_sequence
                && can_spend_summary_call
            {
                let source = history::summary_source(
                    &projection.omitted,
                    context.thread.context_summary.as_ref(),
                    history::CONTEXT_SUMMARY_SOURCE_TOKENS,
                )?;
                let outcome = compact_context(
                    state,
                    claimed,
                    &mut context,
                    latest_message_sequence,
                    source,
                )
                .await?;
                if outcome.applied {
                    if let Some(summary_tokens) = outcome.new_tokens {
                        steps = steps.saturating_add(1);
                        model_calls = model_calls.saturating_add(1);
                        tokens = tokens.saturating_add(summary_tokens);
                        usage["steps_used"] = json!(steps);
                        usage["model_calls_used"] = json!(model_calls);
                        usage["tokens_used"] = json!(tokens);
                    }
                    let post_compaction = history::fit_context_projection(
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
                    context_input_tokens = post_compaction.input_tokens;
                    context_capacity_tokens = post_compaction.capacity;
                    output_tokens = post_compaction.output_tokens;
                    request_messages = post_compaction.messages;
                }
            }
            usage["context_input_tokens"] = json!(context_input_tokens);
            usage["context_capacity_tokens"] = json!(context_capacity_tokens);
            persist_usage(
                state,
                claimed,
                &usage,
                steps,
                model_calls,
                tool_calls,
                tokens,
            )
            .await?;
            let context_manifest_json = Some(serde_json::to_value(context::build_manifest(
                &context,
                &request_messages,
                &skills,
                latest_message_sequence,
            )?)?);
            let mut request = ChatRequest {
                messages: request_messages,
                vision_images: Vec::new(),
                temperature: Some(0.2),
                max_tokens: Some(output_tokens),
                response_format: None,
                stream_include_usage: true,
                cache_scope_key: Some(format!("agent-thread:{}", context.thread.id)),
                cache_breakpoints: Vec::new(),
                cache_mode: None,
                cache_system_prefix: true,
                cache_user_prefix_len: None,
                trace_context: Some(LlmTraceContext {
                    project_id: context.scope.project_id,
                    root_project_id: None,
                    workflow: "agent".into(),
                    step_key: format!("run:{}:model:{model_calls}", claimed.run.id),
                    generation_run_id: None,
                    workflow_run_id: None,
                    call_key: Some(format!("model-call:{model_calls}")),
                    input_fingerprint_sha256: None,
                    context_plan_json: None,
                    context_pack_refs_json: None,
                }),
                context_manifest_json,
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
                if remaining <= input_tokens {
                    state.wait_agent_worker_run(claimed, "budget").await?;
                    return Ok(());
                }
                request.max_tokens = Some(output_tokens.min((remaining - input_tokens) as usize));
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
            agent_protocol_from_request_snapshot(&request.snapshot_value()?).to_string();
        let response: ChatResponse = if replayed_model_execution {
            serde_json::from_value(model_execution.result_json.clone().ok_or_else(|| {
                AppError::InvalidState {
                    current: format!(
                        "Agent model execution {} succeeded 但缺少 result_json",
                        model_execution.id
                    ),
                    operation: "恢复 Agent model execution".into(),
                }
            })?)?
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
                format!("agent-stream:{}:{}", claimed.run.id, model_calls);
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
                            UsageCounters {
                                steps,
                                model_calls,
                                tool_calls,
                                tokens,
                            },
                            &error,
                        )
                        .await?;
                        return Err(error);
                    }
                },
            };
            let response = match consume_model_stream(
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
                    record_provider_failure(
                        state,
                        claimed,
                        &mut usage,
                        UsageCounters {
                            steps,
                            model_calls,
                            tool_calls,
                            tokens,
                        },
                        &error,
                    )
                    .await?;
                    return Err(error);
                }
            };
            execution_application
                .update_worker_execution(WorkerExecutionUpdate {
                    run_id: claimed.run.id,
                    lease_token: claimed.lease_token.clone(),
                    execution_id: model_execution.id,
                    status: "succeeded".into(),
                    result_json: Some(serde_json::to_value(&response)?),
                    result_summary: Some("Agent model call completed".into()),
                    result_refs_json: None,
                    error_message: None,
                })
                .await?;
            response
        };
        a4n_llm_provider_contract::validate_finish_reason(response.finish_reason.as_deref())?;
        // A succeeded durable execution is a replay after a worker restart;
        // its usage was already persisted before the crash. Counting it
        // again would let retries silently exceed token budgets.
        if !replayed_model_execution {
            if let Some(info) = response.usage.as_ref() {
                tokens = tokens.saturating_add(info.total_tokens as u64);
            } else if budget.max_tokens.is_some() {
                return Err(AppError::InvalidState {
                    current: format!(
                        "Agent run {} 的模型响应缺少 usage，无法执行 max_tokens 预算",
                        claimed.run.id
                    ),
                    operation: "执行 Agent model call".into(),
                });
            }
        }
        usage["tokens_used"] = json!(tokens);
        persist_usage(
            state,
            claimed,
            &usage,
            steps,
            model_calls,
            tool_calls,
            tokens,
        )
        .await?;
        let decision = decode_agent_response(&response, &request_protocol, model_execution.id)?;
        let (assistant_content, calls, is_final) = match decision {
            AgentModelDecision::Final { reply } => (reply, Vec::new(), true),
            AgentModelDecision::Think { content } => (content, Vec::new(), false),
            AgentModelDecision::ToolCalls { content, calls } => (content, calls, false),
        };
        if is_final {
            usage["phase"] = json!(a4n_api_contract::agent::AGENT_PHASE_FINAL);
            usage["tokens_used"] = json!(tokens);
            usage["tool_calls_used"] = json!(tool_calls);
            persist_usage(
                state,
                claimed,
                &usage,
                steps,
                model_calls,
                tool_calls,
                tokens,
            )
            .await?;
            complete_run(
                state,
                claimed,
                assistant_content,
                execution_input_message_id(&model_execution)?,
            )
            .await?;
            return Ok(());
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
        if finalized.is_none() {
            message_application
                .append_assistant_tool_message(claimed, assistant_content, tool_calls_payload)
                .await?;
        }
        messages.push(assistant);
        if execute_tool_calls(
            ToolBatchContext {
                state,
                claimed,
                context: &context,
                executor: &executor,
                budget: &budget,
                prepared_calls: &mut prepared_calls,
                discovery_cache: &discovery_cache,
                retry_counts: &retry_counts,
                discovered_tools: &mut discovered_tools,
                messages: &mut messages,
                usage: &mut usage,
                tool_calls: &mut tool_calls,
                steps,
                model_calls,
                tokens,
            },
            calls,
        )
        .await?
            == RunProgress::Released
        {
            return Ok(());
        }
        usage["tool_calls_used"] = json!(tool_calls);
        usage["tokens_used"] = json!(tokens);
        state
            .update_agent_worker_usage(claimed, usage.clone())
            .await?;
    }
}
