use crate::host::AgentRuntimeHost;
use a4n_agent_application::agent::{
    AgentApplication, AgentApprovalExpiryApplicationPort, AgentEditorClaimApplicationPort,
    AgentEditorResponseApplicationPort, AgentEditorWaitApplicationPort,
    AgentExternalTaskWaitApplicationPort, AgentGenerationCancelApplicationPort, AgentRunContext,
    AgentRunLeaseRecoveryApplicationPort, AgentWorkerActivationApplicationPort,
    AgentWorkerApprovalApplicationPort, AgentWorkerBudgetApplicationPort,
    AgentWorkerClaimApplicationPort, AgentWorkerCompletionApplicationPort,
    AgentWorkerDelegationApplicationPort, AgentWorkerExecutionApplicationPort,
    AgentWorkerFailureApplicationPort, AgentWorkerHistoryApplicationPort,
    AgentWorkerInboxApplicationPort, AgentWorkerLeaseApplicationPort,
    AgentWorkerMessageApplicationPort, AgentWorkerReadApplicationPort,
    AgentWorkerRecoveryApplicationPort, AgentWorkerRun, AgentWorkflowBindingApplicationPort,
    AgentWorkflowCancelApplicationPort,
};
use a4n_api_contract::runtime_config::RuntimeConfigSnapshot;
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentHostCommandPayload, AgentRunPayload, AgentWorkerContextSummaryRequest,
    AgentWorkerRunLeaseRequest, AgentWorkerRunWaitRequest, AgentWorkerUsageRequest, WorkerLease,
    WorkerRequest,
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
    pub fn worker_approval_application(&self) -> &dyn AgentWorkerApprovalApplicationPort {
        self.application.as_ref()
    }
    pub fn worker_budget_application(&self) -> &dyn AgentWorkerBudgetApplicationPort {
        self.application.as_ref()
    }
    pub fn worker_claim_application(&self) -> &dyn AgentWorkerClaimApplicationPort {
        self.application.as_ref()
    }
    pub fn worker_completion_application(&self) -> &dyn AgentWorkerCompletionApplicationPort {
        self.application.as_ref()
    }
    pub fn worker_delegation_application(&self) -> &dyn AgentWorkerDelegationApplicationPort {
        self.application.as_ref()
    }
    pub fn worker_execution_application(&self) -> &dyn AgentWorkerExecutionApplicationPort {
        self.application.as_ref()
    }
    pub fn worker_failure_application(&self) -> Arc<dyn AgentWorkerFailureApplicationPort> {
        self.application.clone()
    }
    pub fn worker_history_application(&self) -> &dyn AgentWorkerHistoryApplicationPort {
        self.application.as_ref()
    }
    pub fn worker_inbox_application(&self) -> &dyn AgentWorkerInboxApplicationPort {
        self.application.as_ref()
    }
    pub fn worker_lease_application(&self) -> Arc<dyn AgentWorkerLeaseApplicationPort> {
        self.application.clone()
    }
    pub fn worker_message_application(&self) -> &dyn AgentWorkerMessageApplicationPort {
        self.application.as_ref()
    }
    pub fn worker_read_application(&self) -> &dyn AgentWorkerReadApplicationPort {
        self.application.as_ref()
    }
    pub fn worker_recovery_application(&self) -> &dyn AgentWorkerRecoveryApplicationPort {
        self.application.as_ref()
    }
    pub fn workflow_binding_application(&self) -> &dyn AgentWorkflowBindingApplicationPort {
        self.application.as_ref()
    }
    pub fn editor_claim_application(&self) -> &dyn AgentEditorClaimApplicationPort {
        self.application.as_ref()
    }
    pub fn editor_response_application(&self) -> &dyn AgentEditorResponseApplicationPort {
        self.application.as_ref()
    }
    pub fn editor_wait_application(&self) -> &dyn AgentEditorWaitApplicationPort {
        self.application.as_ref()
    }
    pub fn approval_expiry_application(&self) -> &dyn AgentApprovalExpiryApplicationPort {
        self.application.as_ref()
    }
    pub fn run_lease_recovery_application(&self) -> &dyn AgentRunLeaseRecoveryApplicationPort {
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
