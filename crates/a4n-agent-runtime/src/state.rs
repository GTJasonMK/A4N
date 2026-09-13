use crate::host::AgentRuntimeHost;
use a4n_agent_application::agent::{
    AgentApplication, AgentDelegationSnapshot, AgentEditorClaimApplicationPort,
    AgentExecutionSnapshot, AgentExternalTaskWaitApplicationPort,
    AgentGenerationCancelApplicationPort, AgentInboxSnapshot, AgentMessageSnapshot,
    AgentRunContext, AgentRunSnapshot, AgentTurnSnapshot, AgentWorkerActivationApplicationPort,
    AgentWorkerApprovalApplicationPort, AgentWorkerBudgetApplicationPort,
    AgentWorkerBudgetSnapshot, AgentWorkerDelegationApplicationPort,
    AgentWorkerExecutionApplicationPort, AgentWorkerFailureApplicationPort,
    AgentWorkerInboxApplicationPort, AgentWorkerLeaseApplicationPort,
    AgentWorkerMessageApplicationPort, AgentWorkerReadApplicationPort, AgentWorkerRun,
    AgentWorkflowBindingApplicationPort, AgentWorkflowCancelApplicationPort, WorkerExecutionInput,
    WorkerExecutionUpdate,
};
use a4n_api_contract::runtime_config::RuntimeConfigSnapshot;
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentHostCommandPayload, AgentRunPayload, AgentWorkerApprovalCheckRequest,
    AgentWorkerApprovalCheckSnapshotWire, AgentWorkerApprovalRequest, AgentWorkerBudgetRequest,
    AgentWorkerCompletionRequest, AgentWorkerCompletionSnapshotWire,
    AgentWorkerContextSummaryRequest, AgentWorkerDelegationReviewRequest,
    AgentWorkerDelegationReviewSnapshotWire, AgentWorkerDelegationStartRequest,
    AgentWorkerDelegationStartSnapshotWire, AgentWorkerDelegationThreadRequest,
    AgentWorkerDelegationThreadSnapshotWire, AgentWorkerEditorWaitRequest, AgentWorkerEventRequest,
    AgentWorkerExecutionCreateRequest, AgentWorkerExecutionLookupRequest,
    AgentWorkerExecutionResultRequest, AgentWorkerExecutionUpdateRequest,
    AgentWorkerFailureRequest, AgentWorkerHistoryRequest, AgentWorkerHistorySnapshotWire,
    AgentWorkerInboxClaimRequest, AgentWorkerInboxConsumeRequest, AgentWorkerInboxMessageRequest,
    AgentWorkerInboxPersistRequest, AgentWorkerInboxPersistSnapshotWire, AgentWorkerMemoryRequest,
    AgentWorkerMessageOperation, AgentWorkerMessageRequest, AgentWorkerRecoveryRequest,
    AgentWorkerResultRefsRequest, AgentWorkerResultRefsSnapshotWire, AgentWorkerRunLeaseRequest,
    AgentWorkerRunStatusSnapshotWire, AgentWorkerRunWaitRequest, AgentWorkerUsageRequest,
    WorkerLease, WorkerRequest,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify, RwLock};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct AgentRuntimeState {
    application: Arc<AgentApplication>,
    host: Arc<dyn AgentRuntimeHost>,
    agent_worker_cancel: CancellationToken,
    agent_worker_notify: Arc<Notify>,
    agent_admission: Arc<Mutex<()>>,
    agent_run_cancellations: Arc<Mutex<HashMap<i64, AgentRunCancellation>>>,
    agent_runtime_snapshots: Arc<RwLock<HashMap<i64, AgentRuntimeSnapshot>>>,
}

#[derive(Clone, Copy)]
pub struct AgentWorkerExecutionHostPort<'a> {
    state: &'a AgentRuntimeState,
    claimed: &'a AgentWorkerRun,
}

#[derive(Clone, Copy)]
pub(crate) struct AgentWorkerMessageHostPort<'a> {
    state: &'a AgentRuntimeState,
    claimed: &'a AgentWorkerRun,
}

#[derive(Clone, Copy)]
pub(crate) struct AgentWorkerBudgetHostPort<'a> {
    state: &'a AgentRuntimeState,
    claimed: &'a AgentWorkerRun,
}

#[derive(Clone, Copy)]
pub(crate) struct AgentWorkerApprovalHostPort<'a> {
    state: &'a AgentRuntimeState,
    claimed: &'a AgentWorkerRun,
}

#[derive(Clone, Copy)]
pub(crate) struct AgentWorkerFailureHostPort<'a> {
    state: &'a AgentRuntimeState,
    claimed: &'a AgentWorkerRun,
}

#[derive(Clone)]
struct AgentRunCancellation {
    parent_run_id: Option<i64>,
    token: CancellationToken,
}

#[derive(Clone)]
struct AgentRuntimeSnapshot {
    parent_run_id: Option<i64>,
    value: RuntimeConfigSnapshot,
}

impl AgentRuntimeState {
    pub fn new(application: Arc<AgentApplication>, host: Arc<dyn AgentRuntimeHost>) -> Self {
        Self {
            application,
            host,
            agent_worker_cancel: CancellationToken::new(),
            agent_worker_notify: Arc::new(Notify::new()),
            agent_admission: Arc::new(Mutex::new(())),
            agent_run_cancellations: Arc::new(Mutex::new(HashMap::new())),
            agent_runtime_snapshots: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn host(&self) -> &dyn AgentRuntimeHost {
        self.host.as_ref()
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.agent_worker_cancel.clone()
    }

    pub fn notify(&self) -> Arc<Notify> {
        self.agent_worker_notify.clone()
    }

    pub fn trigger_agent_worker(&self) {
        self.agent_worker_notify.notify_one();
    }

    pub async fn lock_agent_admission(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.agent_admission.lock().await
    }

    pub async fn load_agent_run_context(
        &self,
        request: &WorkerRequest<AgentRunPayload>,
    ) -> Result<AgentRunContext, AppError> {
        self.host.load_run_context(request).await
    }

    pub async fn read_worker_history(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<AgentWorkerHistorySnapshotWire, AppError> {
        let request = AgentWorkerHistoryRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent worker history request 无效: {error}"),
            operation: "读取 Agent worker history".into(),
        })?;
        self.host.read_worker_history(&request).await
    }

    pub async fn mark_agent_recovery_required(
        &self,
        claimed: &AgentWorkerRun,
        reason: String,
    ) -> Result<(), AppError> {
        let request = AgentWorkerRecoveryRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
            reason,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent worker recovery request 无效: {error}"),
            operation: "标记 Agent worker recovery".into(),
        })?;
        self.host.mark_recovery_required(&request).await
    }

    pub async fn wait_for_editor(
        &self,
        claimed: &AgentWorkerRun,
        editor_request: &a4n_api_contract::agent::AgentEditorRequest,
    ) -> Result<(), AppError> {
        let request = AgentWorkerEditorWaitRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
            editor_request: editor_request.clone(),
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent editor wait request 无效: {error}"),
            operation: "等待 Agent editor".into(),
        })?;
        self.host.wait_for_editor(&request).await
    }

    fn agent_worker_lease_request(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<AgentWorkerRunLeaseRequest, AppError> {
        let worker_id = claimed
            .run
            .worker_id
            .clone()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("Agent run {} 缺少 worker_id", claimed.run.id),
                operation: "续期 Agent worker lease".into(),
            })?;
        let epoch = u64::try_from(claimed.run.attempt).map_err(|_| AppError::InvalidState {
            current: format!("Agent run {} 的 attempt 无效", claimed.run.id),
            operation: "续期 Agent worker lease".into(),
        })?;
        let expires_at = claimed
            .run
            .lease_expires_at
            .ok_or_else(|| AppError::InvalidState {
                current: format!("Agent run {} 缺少 lease_expires_at", claimed.run.id),
                operation: "续期 Agent worker lease".into(),
            })?;
        let request = AgentWorkerRunLeaseRequest {
            run_id: claimed.run.id,
            lease: WorkerLease {
                worker_id,
                epoch,
                fence_token: claimed.lease_token.clone(),
                expires_at: expires_at.to_string(),
            },
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent worker lease request 无效: {error}"),
            operation: "续期 Agent worker lease".into(),
        })?;
        Ok(request)
    }

    pub async fn renew_agent_worker_lease(&self, claimed: &AgentWorkerRun) -> Result<(), AppError> {
        let request = self.agent_worker_lease_request(claimed)?;
        self.host.renew_worker_lease(&request).await
    }

    pub async fn mark_agent_turn_active(&self, claimed: &AgentWorkerRun) -> Result<(), AppError> {
        let request = self.agent_worker_lease_request(claimed)?;
        self.host.mark_turn_active(&request).await
    }

    pub async fn update_agent_worker_usage(
        &self,
        claimed: &AgentWorkerRun,
        usage_json: Value,
    ) -> Result<(), AppError> {
        let lease = self.agent_worker_lease_request(claimed)?.lease;
        let request = AgentWorkerUsageRequest {
            run_id: claimed.run.id,
            lease,
            usage_json,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent worker usage request 无效: {error}"),
            operation: "更新 Agent worker usage".into(),
        })?;
        self.host.update_run_usage(&request).await
    }

    pub async fn result_refs_for_run(&self, claimed: &AgentWorkerRun) -> Result<Value, AppError> {
        let request = AgentWorkerResultRefsRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent result refs request 无效: {error}"),
            operation: "读取 Agent result refs".into(),
        })?;
        self.host
            .result_refs_for_run(&request)
            .await
            .map(|snapshot: AgentWorkerResultRefsSnapshotWire| snapshot.result_refs_json)
    }

    pub async fn complete_agent_worker_run(
        &self,
        claimed: &AgentWorkerRun,
        content: String,
        result_summary: String,
        result_refs_json: Option<Value>,
        observed_input_message_id: i64,
    ) -> Result<bool, AppError> {
        let request = AgentWorkerCompletionRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
            content,
            result_summary,
            result_refs_json,
            observed_input_message_id,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent completion request 无效: {error}"),
            operation: "完成 Agent worker run".into(),
        })?;
        self.host
            .complete_worker_run(&request)
            .await
            .map(|snapshot: AgentWorkerCompletionSnapshotWire| snapshot.completed)
    }

    async fn approved_agent_execution(
        &self,
        claimed: &AgentWorkerRun,
        execution_id: i64,
    ) -> Result<bool, AppError> {
        let request = AgentWorkerApprovalCheckRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
            execution_id,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent approval check request 无效: {error}"),
            operation: "读取 Agent approval".into(),
        })?;
        self.host
            .approved_execution(&request)
            .await
            .map(|snapshot: AgentWorkerApprovalCheckSnapshotWire| snapshot.approved)
    }

    async fn request_agent_worker_approval(
        &self,
        claimed: &AgentWorkerRun,
        execution_id: i64,
        operation_json: Value,
        resource_preconditions_json: Option<Value>,
        expires_at: Option<chrono::NaiveDateTime>,
    ) -> Result<(), AppError> {
        let request = AgentWorkerApprovalRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
            execution_id,
            operation_json,
            resource_preconditions_json,
            expires_at,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent approval request 无效: {error}"),
            operation: "请求 Agent approval".into(),
        })?;
        self.host.request_approval(&request).await
    }

    pub async fn review_agent_delegation(
        &self,
        worker_request: &WorkerRequest<AgentRunPayload>,
        user_id: i64,
        parent_run_id: Option<i64>,
        input: a4n_api_contract::agent::ReviewAgentDelegationInput,
    ) -> Result<AgentWorkerDelegationReviewSnapshotWire, AppError> {
        if worker_request.payload.worker_run.thread.owner_user_id != user_id {
            return Err(AppError::Forbidden(
                "Agent delegation review user 与 worker run 不一致".into(),
            ));
        }
        let request = AgentWorkerDelegationReviewRequest {
            run_id: worker_request.payload.run_id,
            lease: worker_request.lease.clone(),
            user_id,
            parent_run_id,
            input,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent delegation review request 无效: {error}"),
            operation: "审阅 Agent delegation".into(),
        })?;
        self.host.review_delegation(&request).await
    }

    pub async fn ensure_agent_canonical_subagent_thread(
        &self,
        worker_request: &WorkerRequest<AgentRunPayload>,
        user_id: i64,
        input: a4n_agent_application::agent::EnsureCanonicalSubagentThreadInput,
    ) -> Result<AgentWorkerDelegationThreadSnapshotWire, AppError> {
        if worker_request.payload.worker_run.thread.owner_user_id != user_id {
            return Err(AppError::Forbidden(
                "Agent delegation thread user 与 worker run 不一致".into(),
            ));
        }
        let request = AgentWorkerDelegationThreadRequest {
            run_id: worker_request.payload.run_id,
            lease: worker_request.lease.clone(),
            user_id,
            canonical_binding_key: input.canonical_binding_key,
            capability: input.capability,
            title: input.title,
            scope: input.scope,
            bound_resource: input.bound_resource,
            default_model_config_id: input.default_model_config_id,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent delegation thread request 无效: {error}"),
            operation: "获取 Agent delegation thread".into(),
        })?;
        self.host.ensure_canonical_subagent_thread(&request).await
    }

    pub async fn append_agent_worker_update(
        &self,
        claimed: &AgentWorkerRun,
        kind: &str,
        status: &str,
        public_text: String,
        payload_json: Option<Value>,
    ) -> Result<(), AppError> {
        let request = AgentWorkerEventRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
            kind: kind.to_string(),
            status: status.to_string(),
            public_text,
            payload_json,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent worker event request 无效: {error}"),
            operation: "写入 Agent worker event".into(),
        })?;
        self.host.append_worker_update(&request).await
    }

    pub async fn create_agent_child_run(
        &self,
        claimed: &AgentWorkerRun,
        input: a4n_agent_application::agent::WorkerDelegationInput,
    ) -> Result<a4n_agent_application::agent::WorkerDelegationResult, AppError> {
        let request = AgentWorkerDelegationStartRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
            capability: input.capability,
            objective: input.objective,
            delegation_key: input.delegation_key,
            child_thread_id: input.child_thread_id,
            scope: input.scope,
            success_criteria: input.success_criteria,
            budget_json: input.budget_json,
            parent_execution_id: input.parent_execution_id,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent delegation start request 无效: {error}"),
            operation: "创建 Agent child run".into(),
        })?;
        self.host
            .create_child_run(&request)
            .await
            .map(worker_delegation_result_from_wire)
    }

    pub async fn read_agent_execution_result(
        &self,
        claimed: &AgentWorkerRun,
        input: a4n_api_contract::agent::ReadAgentExecutionResultInput,
    ) -> Result<a4n_api_contract::agent::AgentExecutionResultPage, AppError> {
        let request = AgentWorkerExecutionResultRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
            execution_id: input.execution_id,
            pointer: input.pointer,
            offset: input.offset,
            limit: input.limit,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent execution result request 无效: {error}"),
            operation: "读取 Agent execution result".into(),
        })?;
        self.host.read_execution_result(&request).await
    }

    pub async fn read_agent_memory(
        &self,
        claimed: &AgentWorkerRun,
        input: a4n_api_contract::agent::ReadAgentMemoryInput,
    ) -> Result<a4n_api_contract::agent::AgentMemorySearchResult, AppError> {
        let request = AgentWorkerMemoryRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
            query: input.query,
            limit: input.limit,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent memory request 无效: {error}"),
            operation: "读取 Agent memory".into(),
        })?;
        self.host.read_memory(&request).await
    }

    pub async fn claim_agent_inbox(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<Option<AgentInboxSnapshot>, AppError> {
        let request = AgentWorkerInboxClaimRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
            thread_id: claimed.run.thread_id,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent inbox claim request 无效: {error}"),
            operation: "领取 Agent inbox".into(),
        })?;
        self.host
            .claim_inbox(&request)
            .await
            .map(|snapshot| snapshot.map(Into::into))
    }

    pub async fn consume_agent_inbox(
        &self,
        claimed: &AgentWorkerRun,
        row: &AgentInboxSnapshot,
    ) -> Result<AgentInboxSnapshot, AppError> {
        let claim_token = row
            .claim_token
            .clone()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("inbox {} 缺少 claim token", row.id),
                operation: "消费 Agent inbox".into(),
            })?;
        let request = AgentWorkerInboxConsumeRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
            inbox_id: row.id,
            claim_token,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent inbox consume request 无效: {error}"),
            operation: "消费 Agent inbox".into(),
        })?;
        self.host.consume_inbox(&request).await.map(Into::into)
    }

    pub async fn persist_agent_inbox(
        &self,
        claimed: &AgentWorkerRun,
        row: &AgentInboxSnapshot,
    ) -> Result<(AgentInboxSnapshot, AgentMessageSnapshot), AppError> {
        let claim_token = row
            .claim_token
            .clone()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("inbox {} 缺少 claim token", row.id),
                operation: "持久化 Agent inbox".into(),
            })?;
        let request = AgentWorkerInboxPersistRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
            inbox_id: row.id,
            claim_token,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent inbox persist request 无效: {error}"),
            operation: "持久化 Agent inbox".into(),
        })?;
        let snapshot: AgentWorkerInboxPersistSnapshotWire =
            self.host.persist_inbox(&request).await?;
        Ok((snapshot.inbox.into(), snapshot.message.into()))
    }

    pub async fn load_agent_inbox_message(
        &self,
        claimed: &AgentWorkerRun,
        message_id: i64,
    ) -> Result<AgentMessageSnapshot, AppError> {
        let request = AgentWorkerInboxMessageRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
            message_id,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent inbox message request 无效: {error}"),
            operation: "加载 Agent inbox message".into(),
        })?;
        self.host.load_inbox_message(&request).await.map(Into::into)
    }

    pub async fn agent_worker_run_is_terminal(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<bool, AppError> {
        let request = self.agent_worker_lease_request(claimed)?;
        self.host
            .run_is_terminal(&request)
            .await
            .map(|snapshot: AgentWorkerRunStatusSnapshotWire| snapshot.terminal)
    }

    pub async fn wait_agent_worker_run(
        &self,
        claimed: &AgentWorkerRun,
        reason: &str,
    ) -> Result<(), AppError> {
        let lease = self.agent_worker_lease_request(claimed)?.lease;
        let request = AgentWorkerRunWaitRequest {
            run_id: claimed.run.id,
            lease,
            reason: reason.to_string(),
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent worker wait request 无效: {error}"),
            operation: "等待 Agent worker run".into(),
        })?;
        self.host.wait_run(&request).await
    }

    pub async fn update_agent_context_summary(
        &self,
        claimed: &AgentWorkerRun,
        context_summary: Value,
    ) -> Result<(), AppError> {
        let lease = self.agent_worker_lease_request(claimed)?.lease;
        let request = AgentWorkerContextSummaryRequest {
            run_id: claimed.run.id,
            lease,
            context_summary,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent context summary request 无效: {error}"),
            operation: "更新 Agent context summary".into(),
        })?;
        self.host.update_context_summary(&request).await
    }

    pub async fn worker_execution_by_key(
        &self,
        claimed: &AgentWorkerRun,
        execution_key: &str,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        let request = AgentWorkerExecutionLookupRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
            execution_key: execution_key.to_string(),
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent execution lookup request 无效: {error}"),
            operation: "读取 Agent execution".into(),
        })?;
        self.host
            .worker_execution_by_key(&request)
            .await
            .map(|snapshot| snapshot.map(Into::into))
    }

    pub async fn create_prepared_execution(
        &self,
        claimed: &AgentWorkerRun,
        input: WorkerExecutionInput,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        if input.run_id != claimed.run.id || input.lease_token != claimed.lease_token {
            return Err(AppError::InvalidState {
                current: format!(
                    "Agent execution input 与 run {} 的 lease 不匹配",
                    claimed.run.id
                ),
                operation: "创建 Agent execution".into(),
            });
        }
        let request = AgentWorkerExecutionCreateRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
            execution_key: input.execution_key,
            execution_kind: input.execution_kind,
            tool_name: input.tool_name,
            effect: input.effect,
            replay_policy: input.replay_policy,
            arguments_json: input.arguments_json,
            resource_preconditions_json: input.resource_preconditions_json,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent execution create request 无效: {error}"),
            operation: "创建 Agent execution".into(),
        })?;
        self.host
            .create_prepared_execution(&request)
            .await
            .map(|snapshot| snapshot.map(Into::into))
    }

    pub async fn update_worker_execution(
        &self,
        claimed: &AgentWorkerRun,
        input: WorkerExecutionUpdate,
    ) -> Result<AgentExecutionSnapshot, AppError> {
        if input.run_id != claimed.run.id || input.lease_token != claimed.lease_token {
            return Err(AppError::InvalidState {
                current: format!(
                    "Agent execution input 与 run {} 的 lease 不匹配",
                    claimed.run.id
                ),
                operation: "更新 Agent execution".into(),
            });
        }
        let request = AgentWorkerExecutionUpdateRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
            execution_id: input.execution_id,
            status: input.status,
            result_json: input.result_json,
            result_summary: input.result_summary,
            result_refs_json: input.result_refs_json,
            error_message: input.error_message,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent execution update request 无效: {error}"),
            operation: "更新 Agent execution".into(),
        })?;
        self.host
            .update_worker_execution(&request)
            .await
            .map(Into::into)
    }

    pub async fn apply_agent_worker_message(
        &self,
        request: &AgentWorkerMessageRequest,
    ) -> Result<Option<AgentMessageSnapshot>, AppError> {
        self.host
            .apply_message_request(request)
            .await
            .map(|snapshot| snapshot.map(Into::into))
    }

    pub(crate) async fn worker_budget_snapshot(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<AgentWorkerBudgetSnapshot, AppError> {
        let request = AgentWorkerBudgetRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent budget request 无效: {error}"),
            operation: "读取 Agent budget".into(),
        })?;
        self.host
            .worker_budget_snapshot(&request)
            .await
            .map(Into::into)
    }

    pub(crate) async fn fail_worker_run(
        &self,
        claimed: &AgentWorkerRun,
        error_message: String,
    ) -> Result<(), AppError> {
        let request = AgentWorkerFailureRequest {
            run_id: claimed.run.id,
            lease: self.agent_worker_lease_request(claimed)?.lease,
            error_message,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent failure request 无效: {error}"),
            operation: "失败 Agent worker run".into(),
        })?;
        self.host.fail_worker_run(&request).await
    }

    pub async fn authorize_agent_host_command(
        &self,
        request: &WorkerRequest<AgentHostCommandPayload>,
    ) -> Result<i64, AppError> {
        self.application
            .authorize_worker_host_command(request)
            .await
    }

    pub async fn dispatch_agent_host_command(
        &self,
        user_id: i64,
        command: String,
        arguments: Value,
        worker_request: &WorkerRequest<AgentRunPayload>,
    ) -> Result<Value, AppError> {
        let payload = AgentHostCommandPayload {
            run_id: worker_request.payload.run_id,
            command: command.clone(),
            arguments,
            runtime_config_snapshot: worker_request.payload.runtime_config_snapshot.clone(),
        };
        payload.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent Host command payload 无效: {error}"),
            operation: "构造 Agent Host command request".into(),
        })?;
        let request = WorkerRequest {
            schema_version: worker_request.schema_version,
            request_id: format!(
                "{}:host:{}",
                worker_request.request_id,
                uuid::Uuid::new_v4()
            ),
            trace_id: worker_request.trace_id.clone(),
            principal: worker_request.principal.clone(),
            operation_id: a4n_worker_contract::AGENT_HOST_COMMAND_OPERATION.into(),
            idempotency_key: format!("{}:host:{}", worker_request.idempotency_key, command),
            project_id: worker_request.project_id,
            root_project_id: worker_request.root_project_id,
            target_revision: worker_request.target_revision,
            runtime_config_snapshot_ref: worker_request.runtime_config_snapshot_ref.clone(),
            lease: worker_request.lease.clone(),
            payload,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent Host command request 无效: {error}"),
            operation: "构造 Agent Host command request".into(),
        })?;
        self.host
            .execute_command(user_id, command, request.payload.arguments.clone(), request)
            .await
    }

    pub async fn store_agent_runtime_snapshot(
        &self,
        run_id: i64,
        parent_run_id: Option<i64>,
        value: RuntimeConfigSnapshot,
    ) {
        self.agent_runtime_snapshots.write().await.insert(
            run_id,
            AgentRuntimeSnapshot {
                parent_run_id,
                value,
            },
        );
    }

    pub async fn agent_runtime_snapshot(&self, run_id: i64) -> Option<RuntimeConfigSnapshot> {
        self.agent_runtime_snapshots
            .read()
            .await
            .get(&run_id)
            .map(|snapshot| snapshot.value.clone())
    }

    pub async fn take_agent_runtime_snapshot(&self, run_id: i64) -> Option<RuntimeConfigSnapshot> {
        self.agent_runtime_snapshots
            .write()
            .await
            .remove(&run_id)
            .map(|snapshot| snapshot.value)
    }

    pub async fn remove_agent_runtime_snapshot_tree(&self, run_id: i64) {
        let mut snapshots = self.agent_runtime_snapshots.write().await;
        let mut pending = vec![run_id];
        while let Some(current) = pending.pop() {
            let children = snapshots
                .iter()
                .filter_map(|(id, snapshot)| {
                    (snapshot.parent_run_id == Some(current)).then_some(*id)
                })
                .collect::<Vec<_>>();
            snapshots.remove(&current);
            pending.extend(children);
        }
    }

    pub async fn register_agent_run_cancellation(
        &self,
        run_id: i64,
        parent_run_id: Option<i64>,
    ) -> CancellationToken {
        let token = CancellationToken::new();
        self.agent_run_cancellations.lock().await.insert(
            run_id,
            AgentRunCancellation {
                parent_run_id,
                token: token.clone(),
            },
        );
        token
    }

    pub async fn cancel_agent_run_tree(&self, run_id: i64) {
        let cancellations = self.agent_run_cancellations.lock().await;
        let mut pending = vec![run_id];
        while let Some(current) = pending.pop() {
            if let Some(entry) = cancellations.get(&current) {
                entry.token.cancel();
            }
            pending.extend(
                cancellations.iter().filter_map(|(id, entry)| {
                    (entry.parent_run_id == Some(current)).then_some(*id)
                }),
            );
        }
    }

    pub async fn unregister_agent_run_cancellation(&self, run_id: i64) {
        self.agent_run_cancellations.lock().await.remove(&run_id);
    }

    pub async fn shutdown(&self) {
        self.agent_worker_cancel.cancel();
        self.agent_worker_notify.notify_waiters();
    }

    pub fn worker_activation_application(&self) -> &dyn AgentWorkerActivationApplicationPort {
        self.application.as_ref()
    }
    pub(crate) fn worker_approval_application<'a>(
        &'a self,
        claimed: &'a AgentWorkerRun,
    ) -> AgentWorkerApprovalHostPort<'a> {
        AgentWorkerApprovalHostPort {
            state: self,
            claimed,
        }
    }
    pub(crate) fn worker_budget_application<'a>(
        &'a self,
        claimed: &'a AgentWorkerRun,
    ) -> AgentWorkerBudgetHostPort<'a> {
        AgentWorkerBudgetHostPort {
            state: self,
            claimed,
        }
    }
    pub fn worker_delegation_application(&self) -> &dyn AgentWorkerDelegationApplicationPort {
        self.application.as_ref()
    }
    pub fn worker_execution_application<'a>(
        &'a self,
        claimed: &'a AgentWorkerRun,
    ) -> AgentWorkerExecutionHostPort<'a> {
        AgentWorkerExecutionHostPort {
            state: self,
            claimed,
        }
    }
    pub(crate) fn worker_failure_application<'a>(
        &'a self,
        claimed: &'a AgentWorkerRun,
    ) -> AgentWorkerFailureHostPort<'a> {
        AgentWorkerFailureHostPort {
            state: self,
            claimed,
        }
    }
    pub fn worker_inbox_application(&self) -> &dyn AgentWorkerInboxApplicationPort {
        self.application.as_ref()
    }
    pub fn worker_lease_application(&self) -> Arc<dyn AgentWorkerLeaseApplicationPort> {
        self.application.clone()
    }
    pub(crate) fn worker_message_application<'a>(
        &'a self,
        claimed: &'a AgentWorkerRun,
    ) -> AgentWorkerMessageHostPort<'a> {
        AgentWorkerMessageHostPort {
            state: self,
            claimed,
        }
    }
    pub fn worker_read_application(&self) -> &dyn AgentWorkerReadApplicationPort {
        self.application.as_ref()
    }
    pub fn workflow_binding_application(&self) -> &dyn AgentWorkflowBindingApplicationPort {
        self.application.as_ref()
    }
    pub fn editor_claim_application(&self) -> &dyn AgentEditorClaimApplicationPort {
        self.application.as_ref()
    }
    pub fn external_task_wait_application(&self) -> &dyn AgentExternalTaskWaitApplicationPort {
        self.application.as_ref()
    }
    pub fn generation_cancel_application(&self) -> &dyn AgentGenerationCancelApplicationPort {
        self.application.as_ref()
    }
    pub fn workflow_cancel_application(&self) -> &dyn AgentWorkflowCancelApplicationPort {
        self.application.as_ref()
    }
}

fn worker_delegation_result_from_wire(
    snapshot: AgentWorkerDelegationStartSnapshotWire,
) -> a4n_agent_application::agent::WorkerDelegationResult {
    a4n_agent_application::agent::WorkerDelegationResult {
        delegation: AgentDelegationSnapshot {
            id: snapshot.delegation.id,
            parent_run_id: snapshot.delegation.parent_run_id,
            parent_execution_id: snapshot.delegation.parent_execution_id,
            child_thread_id: snapshot.delegation.child_thread_id,
            child_turn_id: snapshot.delegation.child_turn_id,
            child_run_id: snapshot.delegation.child_run_id,
            delegation_key: snapshot.delegation.delegation_key,
            capability: snapshot.delegation.capability,
            objective: snapshot.delegation.objective,
            success_criteria_json: snapshot.delegation.success_criteria_json,
            scope_json: snapshot.delegation.scope_json,
            budget_json: snapshot.delegation.budget_json,
            status: snapshot.delegation.status,
            result_refs_json: snapshot.delegation.result_refs_json,
            result_summary: snapshot.delegation.result_summary,
            error_message: snapshot.delegation.error_message,
        },
        child_turn: AgentTurnSnapshot {
            id: snapshot.child_turn.id,
            thread_id: snapshot.child_turn.thread_id,
            input_message_id: snapshot.child_turn.input_message_id,
            execution_mode: snapshot.child_turn.execution_mode,
            goal: snapshot.child_turn.goal,
            model_override_json: snapshot.child_turn.model_override_json,
        },
        child_run: AgentRunSnapshot {
            id: snapshot.child_run.id,
            thread_id: snapshot.child_run.thread_id,
            turn_id: snapshot.child_run.turn_id,
            parent_run_id: snapshot.child_run.parent_run_id,
            attempt: snapshot.child_run.attempt,
            status: snapshot.child_run.status,
            budget_json: snapshot.child_run.budget_json,
            usage_json: snapshot.child_run.usage_json,
            worker_id: snapshot.child_run.worker_id,
            lease_token: snapshot.child_run.lease_token,
            lease_expires_at: snapshot.child_run.lease_expires_at,
            created_at: snapshot.child_run.created_at,
            started_at: snapshot.child_run.started_at,
        },
        child_message: AgentMessageSnapshot {
            id: snapshot.child_message.id,
            thread_id: snapshot.child_message.thread_id,
            turn_id: snapshot.child_message.turn_id,
            run_id: snapshot.child_message.run_id,
            role: snapshot.child_message.role,
            content: snapshot.child_message.content,
            payload_json: snapshot.child_message.payload_json,
        },
    }
}

#[async_trait::async_trait]
impl AgentWorkerExecutionApplicationPort for AgentWorkerExecutionHostPort<'_> {
    async fn worker_execution_by_key(
        &self,
        run_id: i64,
        key: &str,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        if run_id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: format!(
                    "Agent execution run_id 与 claimed run {} 不一致",
                    self.claimed.run.id
                ),
                operation: "读取 Agent execution".into(),
            });
        }
        self.state.worker_execution_by_key(self.claimed, key).await
    }

    async fn create_prepared_execution(
        &self,
        input: WorkerExecutionInput,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        self.state
            .create_prepared_execution(self.claimed, input)
            .await
    }

    async fn update_worker_execution(
        &self,
        input: WorkerExecutionUpdate,
    ) -> Result<AgentExecutionSnapshot, AppError> {
        self.state
            .update_worker_execution(self.claimed, input)
            .await
    }

    async fn worker_execution_by_key_request(
        &self,
        request: &AgentWorkerExecutionLookupRequest,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        self.state
            .host
            .worker_execution_by_key(request)
            .await
            .map(|snapshot| snapshot.map(Into::into))
    }

    async fn create_prepared_execution_request(
        &self,
        request: &AgentWorkerExecutionCreateRequest,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        self.state
            .host
            .create_prepared_execution(request)
            .await
            .map(|snapshot| snapshot.map(Into::into))
    }

    async fn update_worker_execution_request(
        &self,
        request: &AgentWorkerExecutionUpdateRequest,
    ) -> Result<AgentExecutionSnapshot, AppError> {
        self.state
            .host
            .update_worker_execution(request)
            .await
            .map(Into::into)
    }
}

#[async_trait::async_trait]
impl AgentWorkerBudgetApplicationPort for AgentWorkerBudgetHostPort<'_> {
    async fn worker_budget_snapshot(
        &self,
        run_id: i64,
    ) -> Result<AgentWorkerBudgetSnapshot, AppError> {
        if run_id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: format!(
                    "Agent budget run_id 与 claimed run {} 不一致",
                    self.claimed.run.id
                ),
                operation: "读取 Agent budget".into(),
            });
        }
        self.state.worker_budget_snapshot(self.claimed).await
    }

    async fn worker_budget_snapshot_request(
        &self,
        request: &AgentWorkerBudgetRequest,
    ) -> Result<AgentWorkerBudgetSnapshot, AppError> {
        self.state
            .host
            .worker_budget_snapshot(request)
            .await
            .map(Into::into)
    }
}

#[async_trait::async_trait]
impl AgentWorkerApprovalApplicationPort for AgentWorkerApprovalHostPort<'_> {
    async fn approved_execution(
        &self,
        thread_id: i64,
        execution_id: i64,
    ) -> Result<bool, AppError> {
        if thread_id != self.claimed.run.thread_id {
            return Err(AppError::InvalidState {
                current: "Agent approval thread_id 与 claimed run 不一致".into(),
                operation: "读取 Agent approval".into(),
            });
        }
        self.state
            .approved_agent_execution(self.claimed, execution_id)
            .await
    }

    async fn approved_execution_request(
        &self,
        request: &AgentWorkerApprovalCheckRequest,
    ) -> Result<AgentWorkerApprovalCheckSnapshotWire, AppError> {
        if request.run_id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: "Agent approval run_id 与 claimed run 不一致".into(),
                operation: "读取 Agent approval request".into(),
            });
        }
        self.state.host.approved_execution(request).await
    }

    async fn request_worker_approval(
        &self,
        claimed: &AgentWorkerRun,
        execution_id: i64,
        operation_json: Value,
        resource_preconditions_json: Option<Value>,
        expires_at: Option<chrono::NaiveDateTime>,
    ) -> Result<(), AppError> {
        if claimed.run.id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: "Agent approval claimed run 不一致".into(),
                operation: "请求 Agent approval".into(),
            });
        }
        self.state
            .request_agent_worker_approval(
                self.claimed,
                execution_id,
                operation_json,
                resource_preconditions_json,
                expires_at,
            )
            .await
    }

    async fn request_worker_approval_request(
        &self,
        request: &AgentWorkerApprovalRequest,
    ) -> Result<(), AppError> {
        if request.run_id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: "Agent approval run_id 与 claimed run 不一致".into(),
                operation: "请求 Agent approval request".into(),
            });
        }
        self.state.host.request_approval(request).await
    }
}

#[async_trait::async_trait]
impl AgentWorkerFailureApplicationPort for AgentWorkerFailureHostPort<'_> {
    async fn fail_child_run(
        &self,
        claimed: &AgentWorkerRun,
        error_message: String,
    ) -> Result<(), AppError> {
        if claimed.run.id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: "Agent failure claimed run 不一致".into(),
                operation: "持久化 Agent worker failure".into(),
            });
        }
        self.state
            .fail_worker_run(self.claimed, error_message)
            .await
    }

    async fn fail_worker_run(&self, request: &AgentWorkerFailureRequest) -> Result<(), AppError> {
        if request.run_id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: format!(
                    "Agent failure run_id 与 claimed run {} 不一致",
                    self.claimed.run.id
                ),
                operation: "持久化 Agent worker failure request".into(),
            });
        }
        self.state.host.fail_worker_run(request).await
    }
}

impl AgentWorkerMessageHostPort<'_> {
    fn request(
        &self,
        operation: AgentWorkerMessageOperation,
    ) -> Result<AgentWorkerMessageRequest, AppError> {
        let request = AgentWorkerMessageRequest {
            run_id: self.claimed.run.id,
            lease: self.state.agent_worker_lease_request(self.claimed)?.lease,
            operation,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent message request 无效: {error}"),
            operation: "构造 Agent message request".into(),
        })?;
        Ok(request)
    }

    async fn apply(
        &self,
        operation: AgentWorkerMessageOperation,
    ) -> Result<Option<AgentMessageSnapshot>, AppError> {
        let request = self.request(operation)?;
        self.state.apply_agent_worker_message(&request).await
    }

    async fn apply_required(
        &self,
        operation: AgentWorkerMessageOperation,
        operation_name: &str,
    ) -> Result<AgentMessageSnapshot, AppError> {
        self.apply(operation)
            .await?
            .ok_or_else(|| AppError::InvalidState {
                current: "Agent message request 没有返回消息".into(),
                operation: operation_name.into(),
            })
    }
}

#[async_trait::async_trait]
impl AgentWorkerMessageApplicationPort for AgentWorkerMessageHostPort<'_> {
    async fn append_tool_message(
        &self,
        claimed: &AgentWorkerRun,
        execution_id: i64,
        tool_name: &str,
        tool_call_id: Option<&str>,
        result: Value,
    ) -> Result<AgentMessageSnapshot, AppError> {
        if claimed.run.id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: "Agent message claimed run 不一致".into(),
                operation: "写入 Agent tool message".into(),
            });
        }
        self.apply_required(
            AgentWorkerMessageOperation::AppendTool {
                execution_id,
                tool_name: tool_name.to_string(),
                tool_call_id: tool_call_id.map(str::to_string),
                result,
            },
            "写入 Agent tool message",
        )
        .await
    }

    async fn append_assistant_tool_message(
        &self,
        claimed: &AgentWorkerRun,
        content: String,
        tool_calls: Value,
    ) -> Result<AgentMessageSnapshot, AppError> {
        if claimed.run.id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: "Agent message claimed run 不一致".into(),
                operation: "写入 Agent assistant tool message".into(),
            });
        }
        self.apply_required(
            AgentWorkerMessageOperation::AppendAssistantTool {
                content,
                tool_calls,
            },
            "写入 Agent assistant tool message",
        )
        .await
    }

    async fn upsert_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        content: String,
        reasoning_content: Option<String>,
    ) -> Result<AgentMessageSnapshot, AppError> {
        if claimed.run.id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: "Agent message claimed run 不一致".into(),
                operation: "更新 Agent streaming message".into(),
            });
        }
        self.apply_required(
            AgentWorkerMessageOperation::UpsertAssistantStreaming {
                client_message_id,
                content,
                reasoning_content,
            },
            "更新 Agent streaming message",
        )
        .await
    }

    async fn interrupt_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        reason: Option<String>,
    ) -> Result<(), AppError> {
        if claimed.run.id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: "Agent message claimed run 不一致".into(),
                operation: "中断 Agent streaming message".into(),
            });
        }
        self.apply(AgentWorkerMessageOperation::InterruptAssistantStreaming {
            client_message_id,
            reason,
        })
        .await
        .map(|_| ())
    }

    async fn complete_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        content: String,
        tool_calls: Value,
    ) -> Result<Option<AgentMessageSnapshot>, AppError> {
        if claimed.run.id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: "Agent message claimed run 不一致".into(),
                operation: "完成 Agent streaming message".into(),
            });
        }
        self.apply(AgentWorkerMessageOperation::CompleteAssistantStreaming {
            client_message_id,
            content,
            tool_calls,
        })
        .await
    }

    async fn apply_message_request(
        &self,
        request: &AgentWorkerMessageRequest,
    ) -> Result<Option<AgentMessageSnapshot>, AppError> {
        self.state
            .host
            .apply_message_request(request)
            .await
            .map(|snapshot| snapshot.map(Into::into))
    }
}
