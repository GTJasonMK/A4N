use super::*;

/// Executes independent read-only calls with bounded concurrency while keeping
/// write/delegation calls ordered. Every call still goes through the durable
/// execution record and the same lease/approval checks.
pub(crate) struct ToolBatchContext<'a> {
    pub(crate) state: &'a AgentRuntimeState,
    pub(crate) claimed: &'a AgentWorkerRun,
    pub(crate) context: &'a AgentRunContext,
    pub(crate) executor: &'a RuntimeAgentToolExecutor,
    pub(crate) budget: &'a AgentBudget,
    pub(crate) discovery_cache: &'a Arc<Mutex<HashMap<String, Value>>>,
    pub(crate) retry_counts: &'a Arc<Mutex<HashMap<String, u8>>>,
    pub(crate) discovered_tools: &'a mut Vec<String>,
    pub(crate) messages: &'a mut Vec<ChatMessage>,
    pub(crate) usage: &'a mut Value,
}

pub(crate) struct ToolCallContext<'a> {
    state: &'a AgentRuntimeState,
    execution_application: &'a dyn a4n_agent_runtime_contract::AgentWorkerExecutionApplicationPort,
    claimed: &'a AgentWorkerRun,
    context: &'a AgentRunContext,
    executor: &'a RuntimeAgentToolExecutor,
    discovery_cache: Arc<Mutex<HashMap<String, Value>>>,
    retry_counts: Arc<Mutex<HashMap<String, u8>>>,
}

pub(crate) async fn execute_tool_calls(
    batch_context: ToolBatchContext<'_>,
    calls: Vec<ToolCall>,
) -> Result<RunProgress, AppError> {
    let execution_application = batch_context
        .state
        .worker_execution_application(batch_context.claimed);
    let parallelism = usize::from(batch_context.budget.max_parallel_tools.unwrap_or(1).max(1));
    if calls.len() > 1 && calls.iter().any(|call| call.name == "request_agent_input") {
        for call in &calls {
            let Some(execution) =
                prepare_tool_call(&execution_application, batch_context.claimed, call).await?
            else {
                batch_context
                    .state
                    .wait_agent_worker_run(batch_context.claimed, "budget")
                    .await?;
                return Ok(RunProgress::Released);
            };
            if execution.status == "prepared" {
                record_tool_failure(
                    &execution_application,
                    &batch_context
                        .state
                        .worker_message_application(batch_context.claimed),
                    batch_context.claimed,
                    &execution,
                    call,
                    AppError::InvalidArgument(
                        "request_agent_input 必须单独调用；收到用户回复后再决定其他操作".into(),
                    ),
                    batch_context.messages,
                )
                .await?;
            } else {
                append_tool_observation(
                    &batch_context
                        .state
                        .worker_message_application(batch_context.claimed),
                    batch_context.claimed,
                    &execution,
                    call,
                    observation::execution_result(&execution)?,
                    batch_context.messages,
                )
                .await?;
            }
        }
        return Ok(RunProgress::Continue);
    }
    let mut index = 0;
    while index < calls.len() {
        let is_read = tool_descriptor(&calls[index].name)
            .is_ok_and(|descriptor| descriptor.effect == "read")
            && !editor::is_editor_tool(&calls[index].name)
            && !task_wait::is_wait(&calls[index].name)
            && !is_discovery_tool(&calls[index].name);
        let end = if is_read {
            calls[index..]
                .iter()
                .take(parallelism)
                .take_while(|call| {
                    tool_descriptor(&call.name)
                        .map(|descriptor| {
                            descriptor.effect == "read"
                                && !editor::is_editor_tool(&call.name)
                                && !task_wait::is_wait(&call.name)
                                && !is_discovery_tool(&call.name)
                        })
                        .unwrap_or(false)
                })
                .count()
                .max(1)
                + index
        } else {
            index + 1
        };
        let mut batch = Vec::new();
        while index < end {
            let call = &calls[index];
            batch_context.usage["phase"] =
                json!(a4n_api_contract::agent::agent_phase_for_tool(&call.name));
            batch_context.usage["phase_revision"] =
                json!(json_u64(batch_context.usage, "phase_revision")?.saturating_add(1));
            // Preparing is the durable reservation. A restart reconstructs the
            // count from these records, including calls awaiting approval.
            // The shared DB admission also allows waits while tokens are reserved.
            let Some(execution) =
                prepare_tool_call(&execution_application, batch_context.claimed, call).await?
            else {
                break;
            };
            batch.push((call.clone(), execution));
            index += 1;
        }
        persist_usage(
            batch_context.state,
            batch_context.claimed,
            batch_context.usage,
        )
        .await?;
        if batch.is_empty() {
            batch_context
                .state
                .wait_agent_worker_run(batch_context.claimed, "budget")
                .await?;
            return Ok(RunProgress::Released);
        }
        let futures = batch.into_iter().map(|(call, execution)| {
            let execution_context = ToolCallContext {
                state: batch_context.state,
                execution_application: &execution_application,
                claimed: batch_context.claimed,
                context: batch_context.context,
                executor: batch_context.executor,
                discovery_cache: Arc::clone(batch_context.discovery_cache),
                retry_counts: Arc::clone(batch_context.retry_counts),
            };
            Box::pin(async move {
                let mut local_messages = Vec::new();
                let (progress, names) =
                    execute_tool_call(execution_context, call, execution, &mut local_messages)
                        .await?;
                Ok::<_, AppError>((progress, names, local_messages))
            })
        });
        for result in futures::future::join_all(futures).await {
            let (progress, names, local_messages) = result?;
            extend_discovered_tools(
                batch_context.discovered_tools,
                names,
                &batch_context.context.scope,
            )?;
            batch_context.messages.extend(local_messages);
            if progress == RunProgress::Released {
                return Ok(RunProgress::Released);
            }
        }
    }
    Ok(RunProgress::Continue)
}

pub(crate) async fn prepare_tool_call(
    execution_application: &dyn a4n_agent_runtime_contract::AgentWorkerExecutionApplicationPort,
    claimed: &AgentWorkerRun,
    call: &ToolCall,
) -> Result<Option<AgentExecutionSnapshot>, AppError> {
    // Invalid catalog names still get an execution and an explicit error
    // observation; they are never dispatched as a guessed read or write.
    let descriptor = tool_descriptor(&call.name).ok();
    let execution_key = format!("tool-call:{}", call.id);
    let resource_preconditions_json = match execution_application
        .worker_execution_by_key(claimed.run.id, &execution_key)
        .await?
    {
        Some(execution) => execution.resource_preconditions_json,
        None => None,
    };
    execution_application
        .create_prepared_execution(WorkerExecutionInput {
            run_id: claimed.run.id,
            lease_token: claimed.lease_token.clone(),
            execution_key,
            execution_kind: if call.name == "delegate" {
                "delegation"
            } else if task_wait::is_wait(&call.name) {
                "wait"
            } else {
                "tool_call"
            }
            .into(),
            tool_name: Some(call.name.clone()),
            effect: descriptor
                .as_ref()
                .map(|descriptor| descriptor.effect.clone()),
            replay_policy: descriptor
                .and_then(|descriptor| descriptor.agent_policy)
                .map(|policy| enum_string(policy.replay_policy)),
            arguments_json: Some(call.arguments.clone()),
            resource_preconditions_json,
        })
        .await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DelegateToolInput {
    capability: String,
    task: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    project_id: Option<i64>,
    #[serde(default)]
    chapter_id: Option<i64>,
    #[serde(default)]
    success_criteria: Vec<String>,
    #[serde(default)]
    budget: Option<AgentBudget>,
}

pub(crate) async fn append_inbox_messages(
    state: &AgentRuntimeState,
    claimed: &AgentWorkerRun,
    messages: &mut Vec<ChatMessage>,
    loaded_message_ids: &mut HashSet<i64>,
    summary_messages: &mut Vec<AgentMessageSnapshot>,
    pinned_user_contents: &mut Vec<String>,
) -> Result<(), AppError> {
    while let Some(row) = state.claim_agent_inbox(claimed).await? {
        let message_id = inbox_message_id(&row)?;
        if let Some(message_id) = message_id {
            if loaded_message_ids.contains(&message_id) {
                state.consume_agent_inbox(claimed, &row).await?;
                continue;
            }
            let message = state.load_agent_inbox_message(claimed, message_id).await?;
            state.consume_agent_inbox(claimed, &row).await?;
            if message.role == "user" {
                pinned_user_contents.push(message.content.clone());
            }
            messages.push(chat_message_from_model(&message)?);
            summary_messages.push(message.clone());
            loaded_message_ids.insert(message.id);
        } else {
            let (_, message) = state.persist_agent_inbox(claimed, &row).await?;
            if message.role == "user" {
                pinned_user_contents.push(message.content.clone());
            }
            messages.push(chat_message_from_model(&message)?);
            summary_messages.push(message.clone());
            loaded_message_ids.insert(message.id);
        }
    }
    Ok(())
}

pub(crate) fn inbox_message_id(row: &AgentInboxSnapshot) -> Result<Option<i64>, AppError> {
    let Some(value) = row
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get("message_id"))
    else {
        return Ok(None);
    };
    value
        .as_str()
        .ok_or_else(|| AppError::InvalidState {
            current: format!("Agent inbox {} 的 message_id 无效", row.id),
            operation: "加载 Agent inbox message".into(),
        })?
        .parse::<i64>()
        .map(Some)
        .map_err(|error| AppError::InvalidState {
            current: format!("Agent inbox {} 的 message_id 无效: {error}", row.id),
            operation: "加载 Agent inbox message".into(),
        })
}

pub(crate) async fn execute_tool_call(
    execution_context: ToolCallContext<'_>,
    call: ToolCall,
    execution: AgentExecutionSnapshot,
    messages: &mut Vec<ChatMessage>,
) -> Result<(RunProgress, Vec<String>), AppError> {
    let state = execution_context.state;
    let claimed = execution_context.claimed;
    let application = execution_context.execution_application;
    match execute_tool_call_inner(execution_context, &call, &execution, messages).await {
        Err(error) if is_correctable_tool_error(&error) => {
            record_tool_failure(
                application,
                &state.worker_message_application(claimed),
                claimed,
                &execution,
                &call,
                error,
                messages,
            )
            .await
        }
        outcome => outcome,
    }
}

pub fn is_correctable_tool_error(error: &AppError) -> bool {
    matches!(
        error,
        AppError::InvalidArgument(_) | AppError::NotFound(_) | AppError::Forbidden(_)
    )
}

async fn execute_tool_call_inner(
    execution_context: ToolCallContext<'_>,
    call: &ToolCall,
    execution: &AgentExecutionSnapshot,
    messages: &mut Vec<ChatMessage>,
) -> Result<(RunProgress, Vec<String>), AppError> {
    let ToolCallContext {
        state,
        execution_application,
        claimed,
        context,
        executor,
        discovery_cache,
        retry_counts,
    } = execution_context;
    let message_application = state.worker_message_application(claimed);
    let descriptor = match tool_descriptor(&call.name) {
        Ok(descriptor) => descriptor,
        Err(error) => {
            return record_tool_failure(
                execution_application,
                &message_application,
                claimed,
                execution,
                call,
                error,
                messages,
            )
            .await;
        }
    };
    if !a4n_api_contract::agent::capability_allows_tool(&context.profile_id, &call.name) {
        return record_tool_failure(
            execution_application,
            &message_application,
            claimed,
            execution,
            call,
            AppError::Forbidden(format!(
                "Agent capability {} 不允许工具 {}",
                context.profile_id, call.name
            )),
            messages,
        )
        .await;
    }
    if execution.status == "prepared"
        && observation::retry_limit_reached(&retry_counts, &call.name, &call.arguments).await?
    {
        let error_text = "同一工具参数已连续失败 3 次，停止自动重试";
        let value = json!({"error": error_text, "code": "TOOL_RETRY_LIMIT"});
        execution_application
            .update_worker_execution(WorkerExecutionUpdate {
                run_id: claimed.run.id,
                lease_token: claimed.lease_token.clone(),
                execution_id: execution.id,
                status: "failed".into(),
                result_json: Some(value.clone()),
                result_summary: Some(error_text.into()),
                result_refs_json: None,
                error_message: Some(error_text.into()),
            })
            .await?;
        append_tool_observation(
            &message_application,
            claimed,
            execution,
            call,
            value,
            messages,
        )
        .await?;
        return Ok((RunProgress::Continue, Vec::new()));
    }
    let approval_application = state.worker_approval_application(claimed);

    // A completed idempotent execution is a durable replay, not a reason to
    // call an external provider twice after a worker restart.
    if matches!(
        execution.status.as_str(),
        "succeeded" | "failed" | "rejected" | "cancelled"
    ) {
        let value = observation::execution_result(execution)?;
        let discovered = discovered_tool_names(&call.name, &Some(value.clone()))?;
        append_tool_observation(
            &message_application,
            claimed,
            execution,
            call,
            value,
            messages,
        )
        .await?;
        return Ok((RunProgress::Continue, discovered));
    }
    if let Some(key) = discovery_key(&call.name, &call.arguments)
        && let Some(value) = discovery_cache.lock().await.get(&key).cloned()
    {
        execution_application
            .update_worker_execution(WorkerExecutionUpdate {
                run_id: claimed.run.id,
                lease_token: claimed.lease_token.clone(),
                execution_id: execution.id,
                status: "succeeded".into(),
                result_json: Some(value.clone()),
                result_summary: Some("Agent discovery result reused".into()),
                result_refs_json: Some(serde_json::to_value(tool_result_refs(
                    &call.name, &value,
                )?)?),
                error_message: None,
            })
            .await?;
        let discovered = discovered_tool_names(&call.name, &Some(value.clone()))?;
        append_tool_observation(
            &message_application,
            claimed,
            execution,
            call,
            value,
            messages,
        )
        .await?;
        return Ok((RunProgress::Continue, discovered));
    }
    if execution.status == "outcome_unknown" {
        state.wait_agent_worker_run(claimed, "reconcile").await?;
        return Ok((RunProgress::Released, Vec::new()));
    }
    if call.name == "delegate" && execution.status == "running" {
        // A paused parent resumes its existing child instead of returning with
        // a live lease or constructing a duplicate delegation.
        state.wait_agent_worker_run(claimed, "child").await?;
        return Ok((RunProgress::Released, Vec::new()));
    }

    if state
        .worker_budget_application(claimed)
        .worker_budget_snapshot(claimed.run.id)
        .await?
        .max_duration_seconds
        == Some(0)
    {
        state.wait_agent_worker_run(claimed, "budget").await?;
        return Ok((RunProgress::Released, Vec::new()));
    }

    // A dispatched write may have committed before its receipt was saved.
    // Only tools with an explicit safe/idempotent contract may be replayed.
    if matches!(execution.status.as_str(), "dispatching" | "running")
        && descriptor.effect != "read"
        && !matches!(
            execution.replay_policy.as_deref(),
            Some("safe" | "idempotent")
        )
    {
        execution_application
            .update_worker_execution(WorkerExecutionUpdate {
                run_id: claimed.run.id,
                lease_token: claimed.lease_token.clone(),
                execution_id: execution.id,
                status: "outcome_unknown".into(),
                result_json: None,
                result_summary: None,
                result_refs_json: None,
                error_message: Some("工具已派发但缺少提交回执，需要对账，不能重复写入".into()),
            })
            .await?;
        state.wait_agent_worker_run(claimed, "reconcile").await?;
        return Ok((RunProgress::Released, Vec::new()));
    }

    ensure_agent_scope_allows_tool(&context.scope, &call.name)?;
    ensure_argument_scope(&context.scope, &descriptor, &call.arguments)?;
    let delegate_input = if call.name == "delegate" {
        let input = parse_arguments::<DelegateToolInput>(call.arguments.clone(), "delegate")?;
        if input.capability.trim().is_empty() || input.task.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "delegate 的 capability/task 不能为空".into(),
            ));
        }
        if let Some(budget) = &input.budget {
            validate_agent_budget(budget)?;
        }
        Some(input)
    } else {
        None
    };
    if execution.status == "awaiting_user"
        && !approval_application
            .approved_execution(claimed.run.thread_id, execution.id)
            .await?
    {
        state.wait_agent_worker_run(claimed, "approval").await?;
        return Ok((RunProgress::Released, Vec::new()));
    }
    if request_approval_if_needed(
        &approval_application,
        claimed,
        context,
        call,
        execution,
        &descriptor.effect,
    )
    .await?
    {
        return Ok((RunProgress::Released, Vec::new()));
    }

    if call.name == "request_agent_input" {
        let input = parse_arguments::<a4n_api_contract::agent::RequestAgentInput>(
            call.arguments.clone(),
            &call.name,
        )?;
        input.validate().map_err(AppError::InvalidArgument)?;
        let value = json!({"question": input.question, "waiting_for_user": true});
        let observation = observation::from_result(execution.id, &call.name, value)?;
        state
            .append_question(claimed, execution.id, input.question, observation)
            .await?;
        return Ok((RunProgress::Released, Vec::new()));
    }

    if let Some(input) = delegate_input {
        let parent_budget: AgentBudget =
            serde_json::from_value(context.budget.clone()).map_err(|error| {
                AppError::InvalidState {
                    current: format!("Agent run {} budget_json 无效: {error}", claimed.run.id),
                    operation: "执行 Agent delegation".into(),
                }
            })?;
        enforce_delegation_limits(
            &context.delegations,
            context.run.id,
            context.delegation_depth,
            &parent_budget,
        )?;
        let child_budget =
            child_budget_for_delegation(input.budget.unwrap_or_default(), &parent_budget)?;
        let capability = input.capability.trim().to_string();
        let task = input.task.trim().to_string();
        let project_id = input.project_id.or(context.scope.project_id);
        let chapter_id = input.chapter_id.or(context.scope.chapter_id);
        if let Some(project_id) = project_id {
            ensure_project_scope(&context.scope, project_id)?;
        }
        if let Some(chapter_id) = chapter_id {
            ensure_chapter_scope(&context.scope, chapter_id)?;
        }
        if !context.scope.allowed_capabilities.is_empty()
            && !context
                .scope
                .allowed_capabilities
                .iter()
                .any(|item| item == &capability)
        {
            return Err(AppError::Forbidden(format!(
                "Agent scope 不允许委派 capability: {capability}"
            )));
        }
        let binding = binding::canonical_subagent_binding(&capability, project_id, chapter_id)?;
        let child_scope = state
            .host()
            .inherit_agent_delegation_scope(&AgentWorkerDelegationScopeRequest {
                parent_scope: context.scope.clone(),
                child_scope: binding.scope(&capability),
            })
            .await?
            .scope;
        let child_thread = state
            .ensure_agent_canonical_subagent_thread(
                &executor.worker_request,
                context.owner_user_id,
                a4n_agent_runtime_contract::EnsureCanonicalSubagentThreadInput {
                    canonical_binding_key: binding.key,
                    capability: capability.clone(),
                    title: input.title,
                    scope: child_scope.clone(),
                    bound_resource: binding.bound_resource,
                    default_model_config_id: context.default_model_config_id,
                },
            )
            .await?;
        state
            .append_agent_worker_update(
                claimed,
                "delegation_started",
                "published",
                "正在启动子 Agent，等待其完成".into(),
                Some(json!({ "capability": capability.clone(), "task": task.clone() })),
            )
            .await?;
        let _admission = state.lock_agent_admission().await;
        ensure_agent_package_enabled(state).await?;
        let result = state
            .create_agent_child_run(
                claimed,
                WorkerDelegationInput {
                    capability,
                    objective: task,
                    delegation_key: call.id.clone(),
                    child_thread_id: child_thread.id,
                    scope: child_scope,
                    success_criteria: input.success_criteria,
                    budget_json: serde_json::to_value(child_budget)?,
                    parent_execution_id: Some(execution.id),
                    runtime_config_snapshot: None,
                },
            )
            .await?;
        tracing::info!(
            parent_run_id = claimed.run.id,
            delegation_id = result.delegation.id,
            child_run_id = result.child_run.id,
            "Agent child run started"
        );
        return Ok((RunProgress::Released, Vec::new()));
    }

    if context.editor_source.is_some() && editor::needs_editor(&call.name) {
        match editor_request_for_call(context, execution, call) {
            Ok(request) => {
                state.wait_for_editor(claimed, &request).await?;
                return Ok((RunProgress::Released, Vec::new()));
            }
            Err(error) => {
                return record_tool_failure(
                    execution_application,
                    &message_application,
                    claimed,
                    execution,
                    call,
                    error,
                    messages,
                )
                .await;
            }
        }
    }
    if matches!(execution.status.as_str(), "awaiting_user" | "prepared") {
        // Approval releases the lease; reaching here means the operation was
        // approved and the run was claimed again.
        execution_application
            .update_worker_execution(WorkerExecutionUpdate {
                run_id: claimed.run.id,
                lease_token: claimed.lease_token.clone(),
                execution_id: execution.id,
                status: "dispatching".into(),
                result_json: None,
                result_summary: None,
                result_refs_json: None,
                error_message: None,
            })
            .await?;
    }
    execution_application
        .update_worker_execution(WorkerExecutionUpdate {
            run_id: claimed.run.id,
            lease_token: claimed.lease_token.clone(),
            execution_id: execution.id,
            status: "running".into(),
            result_json: None,
            result_summary: None,
            result_refs_json: None,
            error_message: None,
        })
        .await?;
    let approved = descriptor.effect == "read"
        || approval_application
            .approved_execution(claimed.run.thread_id, execution.id)
            .await?;
    let value = match executor
        .execute(
            claimed,
            context,
            &call.name,
            call.arguments.clone(),
            approved,
            Some(execution),
        )
        .await
    {
        Ok(value) => {
            observation::record_success(&retry_counts, &call.name, &call.arguments).await?;
            if task_wait::is_wait(&call.name) && !task_wait::is_terminal(&call.name, &value)? {
                task_wait::release_wait(state, claimed).await?;
                return Ok((RunProgress::Released, Vec::new()));
            }
            execution_application
                .update_worker_execution(WorkerExecutionUpdate {
                    run_id: claimed.run.id,
                    lease_token: claimed.lease_token.clone(),
                    execution_id: execution.id,
                    status: "succeeded".into(),
                    result_json: Some(value.clone()),
                    result_summary: Some("Agent tool completed".into()),
                    result_refs_json: Some(serde_json::to_value(tool_result_refs(
                        &call.name, &value,
                    )?)?),
                    error_message: None,
                })
                .await?;
            if let Some(key) = discovery_key(&call.name, &call.arguments) {
                discovery_cache.lock().await.insert(key, value.clone());
            }
            value
        }
        Err(error) => {
            // Explicit domain-state conflicts can be corrected by reading the
            // current version. A lost lease still cannot persist an observation
            // because the execution update below remains fenced.
            if !is_correctable_tool_error(&error) && !matches!(error, AppError::InvalidState { .. })
            {
                // The remote owner may have committed before its response was
                // lost. Never turn an uncertain write into an ordinary failed
                // observation that invites a new, duplicate tool call.
                if descriptor.effect != "read"
                    && !matches!(
                        execution.replay_policy.as_deref(),
                        Some("safe" | "idempotent")
                    )
                {
                    execution_application
                        .update_worker_execution(WorkerExecutionUpdate {
                            run_id: claimed.run.id,
                            lease_token: claimed.lease_token.clone(),
                            execution_id: execution.id,
                            status: "outcome_unknown".into(),
                            result_json: None,
                            result_summary: None,
                            result_refs_json: None,
                            error_message: Some(error.to_string()),
                        })
                        .await?;
                    state.wait_agent_worker_run(claimed, "reconcile").await?;
                    return Ok((RunProgress::Released, Vec::new()));
                }
                return Err(error);
            }
            let error_text = error.to_string();
            let error_code = observation::tool_error_code(&error).to_string();
            let failure_count =
                observation::record_failure(&retry_counts, &call.name, &call.arguments).await?;
            let (error_text, error_code) = if failure_count >= 3 {
                (
                    "同一工具参数已连续失败 3 次，停止自动重试".to_string(),
                    "TOOL_RETRY_LIMIT".to_string(),
                )
            } else {
                (error_text, error_code)
            };
            let value = json!({"error": error_text, "code": error_code});
            execution_application
                .update_worker_execution(WorkerExecutionUpdate {
                    run_id: claimed.run.id,
                    lease_token: claimed.lease_token.clone(),
                    execution_id: execution.id,
                    status: "failed".into(),
                    result_json: Some(value.clone()),
                    result_summary: None,
                    result_refs_json: None,
                    error_message: Some(error_text.clone()),
                })
                .await?;
            value
        }
    };
    let discovered = discovered_tool_names(&call.name, &Some(value.clone()))?;
    let revision_started = call.name == "review_agent_delegation"
        && value.get("status").and_then(Value::as_str) == Some("running");
    append_tool_observation(
        &message_application,
        claimed,
        execution,
        call,
        value,
        messages,
    )
    .await?;
    if revision_started {
        state.wait_agent_worker_run(claimed, "child").await?;
        return Ok((RunProgress::Released, discovered));
    }
    Ok((RunProgress::Continue, discovered))
}

pub(crate) async fn request_approval_if_needed(
    approval_application: &dyn a4n_agent_runtime_contract::AgentWorkerApprovalApplicationPort,
    claimed: &AgentWorkerRun,
    context: &AgentRunContext,
    call: &ToolCall,
    execution: &AgentExecutionSnapshot,
    effect: &str,
) -> Result<bool, AppError> {
    if !should_request_approval(context, &call.name, effect)?
        || approval_application
            .approved_execution(claimed.run.thread_id, execution.id)
            .await?
    {
        return Ok(false);
    }
    let operation = json!({
        "command": call.name,
        "arguments": call.arguments,
        "expected_effect": effect,
    });
    approval_application
        .request_worker_approval(
            claimed,
            execution.id,
            operation,
            None,
            Some(
                chrono::Utc::now().naive_utc()
                    + chrono::Duration::seconds(WORKER_APPROVAL_TTL_SECONDS),
            ),
        )
        .await?;
    Ok(true)
}

pub async fn append_tool_observation(
    message_application: &dyn a4n_agent_runtime_contract::AgentWorkerMessageApplicationPort,
    claimed: &AgentWorkerRun,
    execution: &AgentExecutionSnapshot,
    call: &ToolCall,
    value: Value,
    messages: &mut Vec<ChatMessage>,
) -> Result<(), AppError> {
    let value = observation::from_result(execution.id, &call.name, value)?;
    message_application
        .append_tool_message(
            claimed,
            execution.id,
            &call.name,
            Some(&call.id),
            value.clone(),
        )
        .await?;
    let mut tool_message = ChatMessage::new(MessageRole::Tool, serde_json::to_string(&value)?);
    tool_message.tool_call_id = Some(call.id.clone());
    messages.push(tool_message);
    Ok(())
}
