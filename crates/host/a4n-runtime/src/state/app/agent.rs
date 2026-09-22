use super::*;

impl AppState {
    /// 启动唯一的 Agent durable-run worker；重复调用不会创建第二个 worker。
    ///
    /// Hosts must call this after installing their Agent Host-command
    /// dispatcher. Keeping startup outside `RuntimeBootstrap::finish` prevents a
    /// recovered Run from reaching a dynamic tool while the Host is still
    /// wiring its command registry.
    pub async fn spawn_agent_worker(&self) -> Result<(), a4n_error::AppError> {
        if self.agent_host_dispatcher.read().await.is_none() {
            return Err(a4n_error::AppError::InvalidState {
                current: "当前 Host 未注册 Agent 动态命令 dispatch".into(),
                operation: "启动 Agent worker".into(),
            });
        }
        let mut slot = self.agent_worker_handle.lock().await;
        if slot.is_some() {
            return Ok(());
        }
        let state = self.clone();
        let cancel = self.agent_worker_cancel.clone();
        let notify = self.agent_worker_notify.clone();
        *slot = Some(tokio::spawn(crate::actions::agent_runtime::run_worker(
            state, cancel, notify,
        )));
        Ok(())
    }

    /// 通知 Agent worker 立即扫描可运行 Run。通知是 best-effort 的，worker 同时
    /// 保留周期扫描，因此进程调度抖动不会丢失已持久化的 Run。
    pub(crate) fn trigger_agent_worker(&self) {
        self.agent_worker_notify.notify_one();
    }

    pub(crate) async fn lock_agent_admission(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.agent_runtime_shared.lock_agent_admission().await
    }

    pub(crate) fn agent_runtime_shared(&self) -> AgentRuntimeShared {
        self.agent_runtime_shared.clone()
    }

    /// Installs the host-owned command dispatch used by dynamically discovered
    /// Agent tools. Hosts may replace the dispatcher during startup, but a
    /// missing dispatcher is surfaced as an error instead of a fake success.
    pub fn set_agent_host_dispatcher(
        &self,
        dispatcher: AgentHostCommandDispatcher,
    ) -> Result<(), a4n_error::AppError> {
        let slot = self.agent_host_dispatcher.clone();
        // Startup calls this synchronously before any user-visible Agent work.
        // Avoid exposing a second async initialization protocol just for one
        // callback slot.
        let mut guard = slot
            .try_write()
            .map_err(|_| a4n_error::AppError::InvalidState {
                current: "Agent host dispatcher 正在使用，无法替换".into(),
                operation: "注册 Agent Host command dispatch".into(),
            })?;
        *guard = Some(dispatcher);
        Ok(())
    }

    pub(crate) async fn dispatch_agent_host_command(
        &self,
        user_id: i64,
        scope: &a4n_api_contract::agent::AgentScope,
        command: String,
        args: Value,
        worker_request: &WorkerEnvelope<AgentRunPayload>,
        execution_id: Option<i64>,
    ) -> Result<Value, a4n_error::AppError> {
        let run_id = worker_request.payload.run_id;
        let runtime_config_snapshot = self
            .agent_runtime_snapshot(run_id)
            .await
            .map(serde_json::to_value)
            .transpose()?;
        let payload = AgentHostCommandPayload {
            run_id,
            turn_id: worker_request.payload.turn_id,
            scope: scope.clone(),
            execution_id,
            command: command.clone(),
            arguments: args,
            runtime_config_snapshot,
        };
        payload
            .validate()
            .map_err(|error| a4n_error::AppError::InvalidState {
                current: format!("Agent Host command payload 无效: {error}"),
                operation: "构造 Agent Host command request".into(),
            })?;
        let host_key = execution_id
            .map(|id| format!("execution:{id}"))
            .unwrap_or_else(|| format!("command:{command}"));
        let request_id = format!("{}:host:{host_key}", worker_request.request_id);
        let request = WorkerEnvelope {
            schema_version: worker_request.schema_version,
            request_id: request_id.clone(),
            trace_id: worker_request.trace_id.clone(),
            principal: worker_request.principal.clone(),
            operation_id: AGENT_HOST_COMMAND_OPERATION.into(),
            idempotency_key: format!("{}:host:{host_key}", worker_request.idempotency_key),
            project_id: worker_request.project_id,
            root_project_id: worker_request.root_project_id,
            target_revision: worker_request.target_revision,
            runtime_config_snapshot_ref: worker_request.runtime_config_snapshot_ref.clone(),
            lease: worker_request.lease.clone(),
            payload: payload.clone(),
        };
        request
            .validate()
            .map_err(|error| a4n_error::AppError::InvalidState {
                current: format!("Agent Host command request 无效: {error}"),
                operation: "构造 Agent Host command request".into(),
            })?;
        let dispatcher = self.agent_host_dispatcher.read().await.clone();
        let Some(dispatcher) = dispatcher else {
            return Err(a4n_error::AppError::InvalidState {
                current: "当前 Host 未注册 Agent 动态命令 dispatch".into(),
                operation: format!("执行 Agent Host command {command}"),
            });
        };
        dispatcher(user_id, command, payload, request)
            .await
            .map_err(|error| a4n_error::AppError::InvalidState {
                current: error,
                operation: "执行 Agent 动态 Host command".into(),
            })
    }

    pub(crate) async fn dispatch_agent_host_command_request(
        &self,
        user_id: i64,
        request: WorkerEnvelope<AgentHostCommandPayload>,
    ) -> Result<Value, a4n_error::AppError> {
        request
            .validate()
            .map_err(|error| a4n_error::AppError::InvalidState {
                current: format!("Agent Host command request 无效: {error}"),
                operation: "执行 Agent Host command".into(),
            })?;
        let command = request.payload.command.clone();
        let dispatcher = self.agent_host_dispatcher.read().await.clone();
        let Some(dispatcher) = dispatcher else {
            return Err(a4n_error::AppError::InvalidState {
                current: "当前 Host 未注册 Agent 动态命令 dispatch".into(),
                operation: format!("执行 Agent Host command {command}"),
            });
        };
        dispatcher(user_id, command, request.payload.clone(), request)
            .await
            .map_err(|error| a4n_error::AppError::InvalidState {
                current: error,
                operation: "执行 Agent 动态 Host command".into(),
            })
    }

    /// Authorizes a Host command at the durable lease boundary.
    pub async fn authorize_agent_host_command(
        &self,
        request: &WorkerEnvelope<AgentHostCommandPayload>,
    ) -> Result<i64, a4n_error::AppError> {
        self.agent_worker_application
            .authorize_worker_host_command(request)
            .await
    }

    pub(crate) async fn cancel_agent_run_tree(&self, run_id: i64) {
        self.agent_runtime_shared
            .cancel_agent_run_tree(run_id)
            .await;
    }

    pub(crate) async fn store_agent_runtime_snapshot(
        &self,
        run_id: i64,
        parent_run_id: Option<i64>,
        snapshot: crate::runtime_config::RuntimeConfigSnapshot,
    ) {
        self.agent_runtime_shared
            .store_agent_runtime_snapshot(run_id, parent_run_id, snapshot)
            .await;
    }

    pub(crate) async fn agent_runtime_snapshot(
        &self,
        run_id: i64,
    ) -> Option<crate::runtime_config::RuntimeConfigSnapshot> {
        self.agent_runtime_shared
            .agent_runtime_snapshot(run_id)
            .await
    }

    pub(crate) async fn remove_agent_runtime_snapshot_tree(&self, run_id: i64) {
        self.agent_runtime_shared
            .remove_agent_runtime_snapshot_tree(run_id)
            .await;
    }

    /// 停止 Agent worker 并等待其退出；可重复调用。
    pub async fn shutdown_agent_worker(&self) {
        self.agent_worker_cancel.cancel();
        self.agent_worker_notify.notify_waiters();
        let handle = self.agent_worker_handle.lock().await.take();
        if let Some(handle) = handle
            && let Err(error) = handle.await
        {
            tracing::warn!(error = %error, "Agent worker 退出异常");
        }
    }

    pub(crate) fn agent_thread_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentThreadMutationApplicationPort {
        self.agent_thread_mutation_application.as_ref()
    }

    pub(crate) fn agent_thread_read_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentThreadReadApplicationPort {
        self.agent_thread_read_application.as_ref()
    }

    pub(crate) fn agent_run_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentRunMutationApplicationPort {
        self.agent_run_mutation_application.as_ref()
    }

    pub(crate) fn agent_run_read_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentRunReadApplicationPort {
        self.agent_run_read_application.as_ref()
    }

    pub(crate) fn agent_delegation_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentDelegationApplicationPort {
        self.agent_delegation_application.as_ref()
    }

    pub(crate) fn agent_tool_read_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentToolReadApplicationPort {
        self.agent_tool_read_application.as_ref()
    }

    pub(crate) fn agent_skill_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentSkillApplicationPort {
        self.agent_skill_application.as_ref()
    }

    pub(crate) fn agent_event_read_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentEventReadApplicationPort {
        self.agent_event_read_application.as_ref()
    }

    pub(crate) fn agent_memory_read_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentMemoryReadApplicationPort {
        self.agent_memory_read_application.as_ref()
    }

    pub(crate) fn agent_memory_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentMemoryMutationApplicationPort {
        self.agent_memory_mutation_application.as_ref()
    }

    pub(crate) fn agent_approval_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentApprovalApplicationPort {
        self.agent_approval_application.as_ref()
    }

    pub(crate) fn agent_resource_access_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentResourceAccessApplicationPort {
        self.agent_resource_access_application.as_ref()
    }

    pub(crate) fn agent_external_task_wait_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentExternalTaskWaitApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_editor_wait_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentEditorWaitApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_editor_claim_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentEditorClaimApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_generation_cancel_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentGenerationCancelApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_workflow_cancel_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkflowCancelApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_budget_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerBudgetApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_approval_expiry_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentApprovalExpiryApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_run_lease_recovery_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentRunLeaseRecoveryApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_lease_application(
        &self,
    ) -> Arc<dyn a4n_agent_runtime_contract::AgentWorkerLeaseApplicationPort> {
        self.agent_worker_application.clone()
    }

    pub(crate) fn agent_worker_claim_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerClaimApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_failure_application(
        &self,
    ) -> Arc<dyn a4n_agent_runtime_contract::AgentWorkerFailureApplicationPort> {
        self.agent_worker_application.clone()
    }

    pub(crate) fn agent_worker_wait_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerWaitApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_read_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerReadApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_context_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerContextApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_execution_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerExecutionApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_approval_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerApprovalApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_completion_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerCompletionApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_delegation_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerDelegationApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_history_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerHistoryApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_inbox_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerInboxApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_message_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerMessageApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_usage_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerUsageApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_editor_response_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentEditorResponseApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_recovery_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerRecoveryApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_activation_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerActivationApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_workflow_binding_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkflowBindingApplicationPort {
        self.agent_worker_application.as_ref()
    }
}
