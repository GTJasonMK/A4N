use crate::host::AgentRuntimeHost;
use a4n_agent_runtime_contract::{
    AgentExecutionSnapshot, AgentInboxSnapshot, AgentMessageSnapshot, AgentRunContext,
    AgentWorkerApprovalApplicationPort, AgentWorkerBudgetApplicationPort,
    AgentWorkerBudgetSnapshot, AgentWorkerExecutionApplicationPort,
    AgentWorkerFailureApplicationPort, AgentWorkerMessageApplicationPort, AgentWorkerRun,
    WorkerExecutionInput, WorkerExecutionUpdate,
};
use a4n_api_contract::agent::{
    AgentScope, AgentSkillContent, AgentSkillSearchResult, LoadAgentSkillInput,
    SearchAgentSkillsInput,
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
    AgentWorkerRunStatusSnapshotWire, AgentWorkerRunWaitRequest, AgentWorkerSkillLoadRequest,
    AgentWorkerSkillSearchRequest, AgentWorkerUsageRequest, WorkerEnvelope, WorkerLease,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, Notify, RwLock};
use tokio_util::sync::CancellationToken;

mod cancellation;
mod ports;
mod shared;
mod snapshot;

pub(super) use cancellation::is_descendant;
pub(crate) use ports::worker_delegation_result_from_wire;
pub use shared::AgentRuntimeShared;
pub(crate) use shared::{AgentRunCancellation, AgentRuntimeSnapshot};

#[derive(Clone)]
pub struct AgentRuntimeState {
    host: Arc<dyn AgentRuntimeHost>,
    agent_worker_cancel: CancellationToken,
    agent_worker_notify: Arc<Notify>,
    shared: AgentRuntimeShared,
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

impl AgentRuntimeState {
    pub fn new(host: Arc<dyn AgentRuntimeHost>) -> Self {
        Self::with_shared(host, AgentRuntimeShared::new())
    }

    pub fn with_shared(host: Arc<dyn AgentRuntimeHost>, shared: AgentRuntimeShared) -> Self {
        Self {
            host,
            agent_worker_cancel: CancellationToken::new(),
            agent_worker_notify: Arc::new(Notify::new()),
            shared,
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
        self.shared.lock_agent_admission().await
    }

    pub async fn load_agent_run_context(
        &self,
        request: &WorkerEnvelope<AgentRunPayload>,
    ) -> Result<AgentRunContext, AppError> {
        self.host
            .load_run_context(request)
            .await
            .map(AgentRunContext::from_wire)
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

    pub async fn search_agent_skills(
        &self,
        claimed: &AgentWorkerRun,
        input: SearchAgentSkillsInput,
    ) -> Result<AgentSkillSearchResult, AppError> {
        let lease = self.agent_worker_lease_request(claimed)?.lease;
        let request = AgentWorkerSkillSearchRequest {
            run_id: claimed.run.id,
            lease,
            input,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent Skill search request 无效: {error}"),
            operation: "搜索 Agent Skill".into(),
        })?;
        self.host.search_agent_skills(&request).await
    }

    pub async fn load_agent_skill(
        &self,
        claimed: &AgentWorkerRun,
        input: LoadAgentSkillInput,
    ) -> Result<AgentSkillContent, AppError> {
        let lease = self.agent_worker_lease_request(claimed)?.lease;
        let request = AgentWorkerSkillLoadRequest {
            run_id: claimed.run.id,
            lease,
            input,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent Skill load request 无效: {error}"),
            operation: "加载 Agent Skill".into(),
        })?;
        self.host.load_agent_skill(&request).await
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
        worker_request: &WorkerEnvelope<AgentRunPayload>,
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
        worker_request: &WorkerEnvelope<AgentRunPayload>,
        user_id: i64,
        input: a4n_agent_runtime_contract::EnsureCanonicalSubagentThreadInput,
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
        input: a4n_agent_runtime_contract::WorkerDelegationInput,
    ) -> Result<a4n_agent_runtime_contract::WorkerDelegationResult, AppError> {
        let runtime_config_snapshot = self.agent_runtime_snapshot(claimed.run.id).await;
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
            runtime_config_snapshot: runtime_config_snapshot.clone(),
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent delegation start request 无效: {error}"),
            operation: "创建 Agent child run".into(),
        })?;
        let result = self
            .host
            .create_child_run(&request)
            .await
            .map(worker_delegation_result_from_wire)?;
        if let Some(runtime_config_snapshot) = runtime_config_snapshot {
            self.store_agent_runtime_snapshot(
                result.child_run.id,
                Some(claimed.run.id),
                runtime_config_snapshot,
            )
            .await;
        }
        Ok(result)
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

    pub async fn dispatch_agent_host_command(
        &self,
        user_id: i64,
        scope: &AgentScope,
        command: String,
        arguments: Value,
        worker_request: &WorkerEnvelope<AgentRunPayload>,
        execution_id: Option<i64>,
    ) -> Result<Value, AppError> {
        let payload = AgentHostCommandPayload {
            run_id: worker_request.payload.run_id,
            turn_id: worker_request.payload.turn_id,
            scope: scope.clone(),
            execution_id,
            command: command.clone(),
            arguments,
            runtime_config_snapshot: worker_request.payload.runtime_config_snapshot.clone(),
        };
        payload.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent Host command payload 无效: {error}"),
            operation: "构造 Agent Host command request".into(),
        })?;
        let host_key = execution_id
            .map(|id| format!("execution:{id}"))
            .unwrap_or_else(|| format!("command:{command}"));
        let request = WorkerEnvelope {
            schema_version: worker_request.schema_version,
            request_id: format!("{}:host:{host_key}", worker_request.request_id),
            trace_id: worker_request.trace_id.clone(),
            principal: worker_request.principal.clone(),
            operation_id: a4n_worker_contract::AGENT_HOST_COMMAND_OPERATION.into(),
            idempotency_key: format!("{}:host:{host_key}", worker_request.idempotency_key),
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
        self.shared
            .store_agent_runtime_snapshot(run_id, parent_run_id, value)
            .await;
    }

    pub async fn agent_runtime_snapshot(&self, run_id: i64) -> Option<RuntimeConfigSnapshot> {
        self.shared.agent_runtime_snapshot(run_id).await
    }

    pub async fn take_agent_runtime_snapshot(&self, run_id: i64) -> Option<RuntimeConfigSnapshot> {
        self.shared.take_agent_runtime_snapshot(run_id).await
    }

    pub async fn remove_agent_runtime_snapshot_tree(&self, run_id: i64) {
        self.shared.remove_agent_runtime_snapshot_tree(run_id).await;
    }

    pub async fn register_agent_run_cancellation(
        &self,
        run_id: i64,
        parent_run_id: Option<i64>,
    ) -> CancellationToken {
        self.shared
            .register_agent_run_cancellation(run_id, parent_run_id)
            .await
    }

    pub async fn cancel_agent_run_tree(&self, run_id: i64) {
        self.shared.cancel_agent_run_tree(run_id).await;
    }

    pub async fn unregister_agent_run_cancellation(&self, run_id: i64) {
        self.shared.unregister_agent_run_cancellation(run_id).await;
    }

    pub async fn shutdown(&self) {
        self.agent_worker_cancel.cancel();
        self.agent_worker_notify.notify_waiters();
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
    pub(crate) fn worker_message_application<'a>(
        &'a self,
        claimed: &'a AgentWorkerRun,
    ) -> AgentWorkerMessageHostPort<'a> {
        AgentWorkerMessageHostPort {
            state: self,
            claimed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AgentRuntimeShared;

    #[tokio::test]
    async fn cancelling_run_tree_only_cancels_descendants() {
        let shared = AgentRuntimeShared::new();
        let root = shared.register_agent_run_cancellation(10, None).await;
        let child = shared.register_agent_run_cancellation(11, Some(10)).await;
        let grandchild = shared.register_agent_run_cancellation(12, Some(11)).await;
        let sibling = shared.register_agent_run_cancellation(20, None).await;

        shared.cancel_agent_run_tree(10).await;

        assert!(root.is_cancelled());
        assert!(child.is_cancelled());
        assert!(grandchild.is_cancelled());
        assert!(!sibling.is_cancelled());
    }
}
