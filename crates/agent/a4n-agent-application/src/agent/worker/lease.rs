//! worker run claim、lease、context 与预算读取。

use super::super::AgentRunContext;
use super::super::AgentWorkerBudgetSnapshot;
use super::super::{AgentApplication, AgentWorkerRun};
use super::*;
use a4n_agent_runtime_contract::AgentRunLeaseRecoveryApplicationPort;
use a4n_agent_runtime_contract::AgentWorkerBudgetApplicationPort;
use a4n_agent_runtime_contract::AgentWorkerClaimApplicationPort;
use a4n_agent_runtime_contract::AgentWorkerContextApplicationPort;
use a4n_agent_runtime_contract::AgentWorkerLeaseApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerBudgetRequest;
use a4n_worker_contract::AgentWorkerContextSummaryRequest;
use a4n_worker_contract::AgentWorkerRunLeaseRequest;
use serde_json::Value;

impl AgentApplication {
    pub(crate) async fn claim_worker_run(
        &self,
        worker_id: &str,
    ) -> Result<Option<AgentWorkerRun>, AppError> {
        let row = agent::claim_runnable_run(self.connection(), worker_id)
            .await
            .map_err(agent_db_error_to_app)?;
        let Some(run_row) = row else {
            return Ok(None);
        };
        let thread_row = agent::thread_by_id(self.connection(), run_row.thread_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::InvalidState {
                current: format!("run {} 缺少 thread {}", run_row.id, run_row.thread_id),
                operation: "领取 Agent run".into(),
            })?;
        let lease_token = run_row
            .lease_token
            .clone()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("run {} 领取后缺少 lease token", run_row.id),
                operation: "领取 Agent run".into(),
            })?;
        Ok(Some(AgentWorkerRun {
            run: run_snapshot(run_row),
            thread: thread_snapshot(thread_row)?,
            lease_token,
        }))
    }

    /// Authorizes a dynamic Host command returned by an external Agent worker.
    /// Every identity and fencing field is checked against the durable Run
    /// before the control plane executes the command on the user's behalf.
    pub async fn authorize_worker_host_command(
        &self,
        request: &WorkerEnvelope<AgentHostCommandPayload>,
    ) -> Result<i64, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(format!("worker request 无效: {error}")))?;
        request.payload.validate().map_err(|error| {
            AppError::InvalidArgument(format!("Agent Host command payload 无效: {error}"))
        })?;
        if request.operation_id != a4n_worker_contract::AGENT_HOST_COMMAND_OPERATION {
            return Err(AppError::InvalidArgument(format!(
                "不支持的 Agent worker operation: {}",
                request.operation_id
            )));
        }
        let run = agent::run_by_id(self.connection(), request.payload.run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| {
                AppError::NotFound(format!("Agent run 不存在: {}", request.payload.run_id))
            })?;
        agent::validate_live_run_lease(
            &run,
            &request.lease.fence_token,
            chrono::Utc::now().naive_utc(),
        )
        .map_err(|error| match error {
            AgentDbError::Conflict(message) => AppError::Forbidden(message),
            other => agent_db_error_to_app(other),
        })?;
        if run.worker_id.as_deref() != Some(request.lease.worker_id.as_str())
            || u64::try_from(run.attempt).ok() != Some(request.lease.epoch)
        {
            return Err(AppError::Forbidden(
                "Agent worker request 与当前 Run lease 不一致".into(),
            ));
        }
        // `expires_at` is an observation captured when the request was built.
        // Renewal legitimately changes it while the fence token and epoch stay
        // stable; validate liveness above against the database as authority.
        let thread = agent::thread_by_id(self.connection(), run.thread_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent thread 不存在: {}", run.thread_id)))?;
        if let Some(execution_id) = request.payload.execution_id {
            let execution = execution::Entity::find_by_id(execution_id)
                .one(self.connection())
                .await?
                .ok_or_else(|| {
                    AppError::NotFound(format!("Agent execution 不存在: {execution_id}"))
                })?;
            if execution.run_id != run.id {
                return Err(AppError::Forbidden(
                    "Agent Host command 的 execution 不属于当前 Run".into(),
                ));
            }
        }
        let mut stored_scope: AgentScope =
            from_json(thread.scope_json.clone(), "agent_thread.scope_json")?;
        self.hydrate_scope_chapter_number(&mut stored_scope).await?;
        let expected_scope = effective_agent_scope(&thread.profile_id, stored_scope)?;
        if !agent_scopes_match_except_chapter_number(&request.payload.scope, &expected_scope) {
            return Err(AppError::Forbidden(
                "Agent Host command 的 scope 与当前 Run 不一致".into(),
            ));
        }
        let user_id = request
            .principal
            .subject
            .parse::<i64>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| AppError::Forbidden("Agent worker principal 无效".into()))?;
        if request.principal.kind != a4n_worker_contract::WorkerPrincipalKind::User
            || thread.owner_user_id != user_id
            || request.project_id != thread.project_id
            || request.target_revision
                != Some(
                    u64::try_from(thread.revision).map_err(|_| AppError::InvalidState {
                        current: format!("Agent thread {} 的 revision 无效", thread.id),
                        operation: "校验 Agent worker request".into(),
                    })?,
                )
        {
            return Err(AppError::Forbidden(
                "Agent worker request 的 principal 或资源版本不匹配".into(),
            ));
        }
        Ok(user_id)
    }

    /// Converts expired leases into the explicit recovery terminal state.
    pub(crate) async fn recover_expired_runs(
        &self,
        limit: u64,
        error_message: Option<&str>,
    ) -> Result<Vec<run::Model>, AppError> {
        let rows = agent::expired_running_runs(self.connection(), limit)
            .await
            .map_err(agent_db_error_to_app)?;
        let mut recovered = Vec::with_capacity(rows.len());
        for row in rows {
            match agent::recover_expired_run(self.connection(), row.id, error_message).await {
                Ok(recovered_row) => recovered.push(recovered_row),
                // Another worker may win the race between the read-only scan
                // and the fenced recovery.  Surface all other failures.
                Err(AgentDbError::Conflict(_)) => {}
                Err(error) => return Err(agent_db_error_to_app(error)),
            }
        }
        Ok(recovered)
    }

    pub(crate) async fn expire_pending_approvals(
        &self,
        limit: u64,
    ) -> Result<Vec<approval::Model>, AppError> {
        agent::expire_pending_approvals(self.connection(), limit)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn delegation_depth_for_run(
        &self,
        run_id: i64,
        parent_run_id: Option<i64>,
    ) -> Result<u8, AppError> {
        let mut depth = 0_u8;
        let mut ancestor = parent_run_id;
        let mut seen = HashSet::new();
        while let Some(parent_id) = ancestor {
            if !seen.insert(parent_id) {
                return Err(AppError::InvalidState {
                    current: format!("Agent run lineage cycle detected: {run_id}"),
                    operation: "读取 Agent run lineage".into(),
                });
            }
            depth = depth.saturating_add(1);
            ancestor = agent::run_by_id(self.connection(), parent_id)
                .await
                .map_err(agent_db_error_to_app)?
                .and_then(|row| row.parent_run_id);
        }
        Ok(depth)
    }

    /// Loads the complete durable context required for one model decision.
    pub(crate) async fn load_run_context(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<AgentRunContext, AppError> {
        let run_row = agent::run_by_id(self.connection(), claimed.run.id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent run 不存在: {}", claimed.run.id)))?;
        if run_row.thread_id != claimed.run.thread_id
            || run_row.turn_id != claimed.run.turn_id
            || run_row.attempt != claimed.run.attempt
            || run_row.worker_id != claimed.run.worker_id
        {
            return Err(AppError::InvalidState {
                current: format!("Agent run {} 的 worker 快照已过期", claimed.run.id),
                operation: "加载 Agent worker context".into(),
            });
        }
        agent::validate_live_run_lease(
            &run_row,
            &claimed.lease_token,
            chrono::Utc::now().naive_utc(),
        )
        .map_err(|error| match error {
            AgentDbError::Conflict(message) => AppError::InvalidState {
                current: message,
                operation: "加载 Agent worker context".into(),
            },
            other => agent_db_error_to_app(other),
        })?;
        let thread_row = agent::thread_by_id(self.connection(), run_row.thread_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| {
                AppError::NotFound(format!("Agent thread 不存在: {}", run_row.thread_id))
            })?;
        let turn_row = turn::Entity::find_by_id(run_row.turn_id)
            .one(self.connection())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("Agent turn 不存在: {}", run_row.turn_id)))?;
        validate_model_override_json(turn_row.model_override_json.as_ref())?;
        let thread = thread_snapshot(thread_row.clone())?;
        let covered_message_id = thread
            .context_summary
            .as_ref()
            .and_then(|summary| summary.get("covered_through_sequence"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let messages = agent::messages_for_context(
            self.connection(),
            run_row.thread_id,
            run_row.id,
            covered_message_id,
        )
        .await
        .map_err(agent_db_error_to_app)?;
        let mut scope = effective_agent_scope(
            &thread_row.profile_id,
            from_json(thread_row.scope_json.clone(), "agent_thread.scope_json")?,
        )?;
        self.hydrate_scope_chapter_number(&mut scope).await?;
        let editor_source_value = messages
            .iter()
            .find(|message| message.id == turn_row.input_message_id)
            .and_then(|message| message.payload_json.as_ref())
            .and_then(|payload| payload.get("editor_source"))
            .cloned();
        let editor_source = match editor_source_value {
            Some(value) => {
                Some(parse_persisted_editor_source(self.connection(), value, &thread_row).await?)
            }
            None => None,
        };
        let delegations = agent::delegations_by_thread(self.connection(), run_row.thread_id)
            .await
            .map_err(agent_db_error_to_app)?;
        let executions = agent::executions_by_runs(self.connection(), vec![run_row.id])
            .await
            .map_err(agent_db_error_to_app)?;
        let runtime_snapshot_required = runtime_snapshot_required(&run_row.usage_json, run_row.id)?;
        let delegation_depth = self
            .delegation_depth_for_run(run_row.id, run_row.parent_run_id)
            .await?;
        Ok(AgentRunContext {
            owner_user_id: thread_row.owner_user_id,
            profile_id: thread_row.profile_id.clone(),
            default_model_config_id: thread_row.default_model_config_id,
            execution_mode: turn_row.execution_mode.clone(),
            goal: turn_row.goal.clone(),
            budget: run_row.budget_json.clone(),
            usage: run_row.usage_json.clone(),
            runtime_snapshot_required,
            delegation_depth,
            run: run_snapshot(run_row),
            thread,
            turn: turn_snapshot(turn_row),
            messages: messages.into_iter().map(message_snapshot).collect(),
            delegations: delegations.into_iter().map(delegation_snapshot).collect(),
            executions: executions.into_iter().map(execution_snapshot).collect(),
            editor_source,
            scope,
        })
    }

    pub(crate) async fn update_context_summary(
        &self,
        claimed: &AgentWorkerRun,
        context_summary: Value,
    ) -> Result<(), AppError> {
        serde_json::from_value::<a4n_agent_runtime_contract::AgentContextSummary>(
            context_summary.clone(),
        )?
        .validate()?;
        agent::update_context_summary_for_run_lease(
            self.connection(),
            claimed.run.id,
            &claimed.lease_token,
            context_summary,
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn update_context_summary_request(
        &self,
        request: &AgentWorkerContextSummaryRequest,
    ) -> Result<(), AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let claimed = self
            .worker_run_for_lease_request(&AgentWorkerRunLeaseRequest {
                run_id: request.run_id,
                lease: request.lease.clone(),
            })
            .await?;
        self.update_context_summary(&claimed, request.context_summary.clone())
            .await
    }

    pub(crate) async fn read_memory_for_run(
        &self,
        context: &AgentRunContext,
        input: a4n_api_contract::agent::ReadAgentMemoryInput,
    ) -> Result<a4n_api_contract::agent::AgentMemorySearchResult, AppError> {
        let thread_row = &context.thread;
        let limit = bounded_limit(input.limit, 20, 100, "limit")? as usize;
        let query = input
            .query
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_lowercase();
        let mut memories =
            agent::list_memory(self.connection(), thread_row.owner_user_id, None, None)
                .await
                .map_err(agent_db_error_to_app)?;
        memories.retain(|row| {
            row.status == "active"
                && match row.scope_kind.as_str() {
                    "user" => row.scope_key == thread_row.owner_user_id.to_string(),
                    "project" => thread_row
                        .project_id
                        .is_some_and(|project_id| row.scope_key == project_id.to_string()),
                    "thread" => row.scope_key == thread_row.id.to_string(),
                    _ => false,
                }
        });
        memories.sort_by(|left, right| {
            (&left.scope_kind, &left.scope_key, &left.memory_key)
                .cmp(&(&right.scope_kind, &right.scope_key, &right.memory_key))
                .then_with(|| right.revision.cmp(&left.revision))
        });
        memories.dedup_by(|left, right| {
            left.scope_kind == right.scope_kind
                && left.scope_key == right.scope_key
                && left.memory_key == right.memory_key
        });
        memories.retain(|row| {
            row.memory_key.to_lowercase().contains(&query)
                || row.content.to_lowercase().contains(&query)
        });
        let total = memories.len();
        memories.truncate(limit);
        Ok(a4n_api_contract::agent::AgentMemorySearchResult {
            memories: memories
                .into_iter()
                .map(memory_view)
                .collect::<Result<_, _>>()?,
            total,
        })
    }

    pub(crate) async fn read_memory_request(
        &self,
        request: &AgentWorkerMemoryRequest,
    ) -> Result<a4n_api_contract::agent::AgentMemorySearchResult, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let claimed = self
            .worker_run_for_lease_request(&AgentWorkerRunLeaseRequest {
                run_id: request.run_id,
                lease: request.lease.clone(),
            })
            .await?;
        let context = self.load_run_context(&claimed).await?;
        self.read_memory_for_run(
            &context,
            a4n_api_contract::agent::ReadAgentMemoryInput {
                query: request.query.clone(),
                limit: request.limit,
            },
        )
        .await
    }

    pub(crate) async fn renew_worker_lease(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<(), AppError> {
        agent::renew_run_lease(self.connection(), claimed.run.id, &claimed.lease_token)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn renew_worker_lease_request(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<(), AppError> {
        self.validate_worker_lease_request(request).await?;
        agent::renew_run_lease(
            self.connection(),
            request.run_id,
            &request.lease.fence_token,
        )
        .await
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn mark_turn_active_request(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<(), AppError> {
        self.validate_worker_lease_request(request).await?;
        agent::mark_turn_active_for_run_lease(
            self.connection(),
            request.run_id,
            &request.lease.fence_token,
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn validate_worker_lease_request(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<run::Model, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let run = agent::run_by_id(self.connection(), request.run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent run 不存在: {}", request.run_id)))?;
        let attempt = i32::try_from(request.lease.epoch)
            .map_err(|_| AppError::InvalidArgument("Agent worker epoch 无效".into()))?;
        if run.worker_id.as_deref() != Some(request.lease.worker_id.as_str())
            || run.attempt != attempt
            || run.lease_token.as_deref() != Some(request.lease.fence_token.as_str())
        {
            return Err(AppError::InvalidState {
                current: format!("Agent run {} 的 worker lease 不匹配", request.run_id),
                operation: "校验 Agent worker lease".into(),
            });
        }
        Ok(run)
    }

    pub(crate) async fn worker_run_for_lease_request(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<AgentWorkerRun, AppError> {
        let run = self.validate_worker_lease_request(request).await?;
        let thread = agent::thread_by_id(self.connection(), run.thread_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent thread 不存在: {}", run.thread_id)))?;
        Ok(AgentWorkerRun {
            run: run_snapshot(run),
            thread: thread_snapshot(thread)?,
            lease_token: request.lease.fence_token.clone(),
        })
    }

    pub(crate) async fn read_execution_result_for_run(
        &self,
        context: &AgentRunContext,
        input: a4n_api_contract::agent::ReadAgentExecutionResultInput,
    ) -> Result<a4n_api_contract::agent::AgentExecutionResultPage, AppError> {
        let limit = bounded_limit(input.limit, 2_000, 2_000, "limit")? as usize;
        let execution_id = parse_id(&input.execution_id, "execution_id")?;
        let execution = execution::Entity::find_by_id(execution_id)
            .one(self.connection())
            .await?
            .filter(|execution| {
                matches!(
                    execution.execution_kind.as_str(),
                    "tool_call" | "delegation"
                )
            })
            .ok_or_else(|| AppError::Forbidden("不能读取此工具结果".into()))?;
        let source_run = agent::run_by_id(self.connection(), execution.run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .filter(|run| run.thread_id == context.thread.id)
            .ok_or_else(|| AppError::Forbidden("不能读取其他 Thread 的工具结果".into()))?;
        self.owned_thread(context.owner_user_id, source_run.thread_id)
            .await?;
        let value = execution
            .result_json
            .as_ref()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("工具结果尚不可用，状态为 {}", execution.status),
                operation: "读取 Agent 工具结果".into(),
            })?;
        let selected = value.pointer(&input.pointer).ok_or_else(|| {
            AppError::InvalidArgument(format!("结果中不存在 JSON pointer: {}", input.pointer))
        })?;
        let text = selected
            .as_str()
            .map(str::to_string)
            .unwrap_or(serde_json::to_string(selected)?);
        let total_chars = text.chars().count();
        let offset = input.offset as usize;
        if offset > total_chars {
            return Err(AppError::InvalidArgument("offset 超出结果长度".into()));
        }
        let page = text.chars().skip(offset).take(limit).collect::<String>();
        let end = offset + page.chars().count();
        Ok(a4n_api_contract::agent::AgentExecutionResultPage {
            execution_id: input.execution_id,
            pointer: input.pointer,
            text: page,
            offset: input.offset,
            total_chars,
            next_offset: (end < total_chars)
                .then(|| u32::try_from(end))
                .transpose()
                .map_err(|_| AppError::InvalidArgument("结果长度超出读取范围".into()))?,
            complete: end == total_chars,
        })
    }

    pub(crate) async fn read_execution_result_request(
        &self,
        request: &AgentWorkerExecutionResultRequest,
    ) -> Result<a4n_api_contract::agent::AgentExecutionResultPage, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let claimed = self
            .worker_run_for_lease_request(&AgentWorkerRunLeaseRequest {
                run_id: request.run_id,
                lease: request.lease.clone(),
            })
            .await?;
        let context = self.load_run_context(&claimed).await?;
        self.read_execution_result_for_run(
            &context,
            a4n_api_contract::agent::ReadAgentExecutionResultInput {
                execution_id: request.execution_id.clone(),
                pointer: request.pointer.clone(),
                offset: request.offset,
                limit: request.limit,
            },
        )
        .await
    }

    pub(crate) async fn result_refs_for_run(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<Value, AppError> {
        self.result_refs_for_run_id(claimed.run.id).await
    }

    pub(super) async fn result_refs_for_run_id(&self, run_id: i64) -> Result<Value, AppError> {
        let executions = agent::executions_by_runs(self.connection(), vec![run_id])
            .await
            .map_err(agent_db_error_to_app)?;
        let mut refs = Vec::<AgentResourceRef>::new();
        for execution in executions
            .into_iter()
            .filter(|execution| execution.status == "succeeded")
        {
            if let Some(value) = execution.result_refs_json {
                let items: Vec<AgentResourceRef> =
                    from_json(value, "agent_execution.result_refs_json")?;
                for item in items {
                    if item.kind == "optimization_source" {
                        continue;
                    }
                    refs.retain(|previous| previous.kind != item.kind || previous.id != item.id);
                    refs.push(item);
                }
            }
        }
        serde_json::to_value(refs).map_err(AppError::from)
    }

    pub(crate) async fn result_refs_for_run_request(
        &self,
        request: &AgentWorkerResultRefsRequest,
    ) -> Result<AgentWorkerResultRefsSnapshotWire, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        self.result_refs_for_run_id(request.run_id)
            .await
            .map(|result_refs_json| AgentWorkerResultRefsSnapshotWire { result_refs_json })
    }

    pub async fn run_budget_snapshot(
        &self,
        run_id: i64,
    ) -> Result<agent::AgentBudgetSnapshot, AppError> {
        agent::run_budget_snapshot(self.connection(), run_id)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn worker_budget_snapshot_request(
        &self,
        request: &AgentWorkerBudgetRequest,
    ) -> Result<AgentWorkerBudgetSnapshot, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        self.worker_budget_snapshot(request.run_id).await
    }
}

#[async_trait::async_trait]
impl AgentWorkerClaimApplicationPort for AgentApplication {
    async fn claim_worker_run(&self, worker_id: &str) -> Result<Option<AgentWorkerRun>, AppError> {
        AgentApplication::claim_worker_run(self, worker_id).await
    }
}

#[async_trait::async_trait]
impl AgentWorkerContextApplicationPort for AgentApplication {
    async fn load_run_context(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<AgentRunContext, AppError> {
        AgentApplication::load_run_context(self, claimed).await
    }

    async fn update_context_summary(
        &self,
        claimed: &AgentWorkerRun,
        context_summary: Value,
    ) -> Result<(), AppError> {
        AgentApplication::update_context_summary(self, claimed, context_summary).await
    }

    async fn update_context_summary_request(
        &self,
        request: &AgentWorkerContextSummaryRequest,
    ) -> Result<(), AppError> {
        AgentApplication::update_context_summary_request(self, request).await
    }
}

#[async_trait::async_trait]
impl AgentWorkerLeaseApplicationPort for AgentApplication {
    async fn renew_worker_lease(&self, claimed: &AgentWorkerRun) -> Result<(), AppError> {
        AgentApplication::renew_worker_lease(self, claimed).await
    }

    async fn renew_worker_lease_request(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<(), AppError> {
        AgentApplication::renew_worker_lease_request(self, request).await
    }
}

impl AgentApplication {
    pub async fn worker_budget_snapshot(
        &self,
        run_id: i64,
    ) -> Result<AgentWorkerBudgetSnapshot, AppError> {
        let snapshot = self.run_budget_snapshot(run_id).await?;
        Ok(AgentWorkerBudgetSnapshot {
            usage: snapshot.usage,
            max_tokens: snapshot.remaining.max_tokens,
            max_duration_seconds: snapshot.remaining.max_duration_seconds,
        })
    }
}

#[async_trait::async_trait]
impl AgentWorkerBudgetApplicationPort for AgentApplication {
    async fn worker_budget_snapshot(
        &self,
        run_id: i64,
    ) -> Result<AgentWorkerBudgetSnapshot, AppError> {
        AgentApplication::worker_budget_snapshot(self, run_id).await
    }

    async fn worker_budget_snapshot_request(
        &self,
        request: &AgentWorkerBudgetRequest,
    ) -> Result<AgentWorkerBudgetSnapshot, AppError> {
        AgentApplication::worker_budget_snapshot_request(self, request).await
    }
}

#[async_trait::async_trait]
impl AgentRunLeaseRecoveryApplicationPort for AgentApplication {
    async fn recover_expired_runs(
        &self,
        limit: u64,
        error_message: Option<&str>,
    ) -> Result<usize, AppError> {
        Ok(
            AgentApplication::recover_expired_runs(self, limit, error_message)
                .await?
                .len(),
        )
    }
}

// Agent worker host command port。

#[async_trait::async_trait]
impl AgentWorkerHostCommandApplicationPort for AgentApplication {
    async fn authorize_worker_host_command(
        &self,
        request: &WorkerEnvelope<AgentHostCommandPayload>,
    ) -> Result<i64, AppError> {
        self.authorize_worker_host_command(request).await
    }
}
