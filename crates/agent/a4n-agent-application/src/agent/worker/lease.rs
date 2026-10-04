//! worker run claim、lease 与 lease 校验。

use super::super::{AgentApplication, AgentWorkerRun};
use super::*;
use a4n_agent_runtime_contract::AgentWorkerClaimApplicationPort;
use a4n_agent_runtime_contract::AgentWorkerLeaseApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerRunLeaseRequest;

impl AgentApplication {
    pub(crate) async fn claim_worker_run(
        &self,
        worker_id: &str,
    ) -> Result<Option<AgentWorkerRun>, AppError> {
        let row = agent::claim_runnable_run(self.executor(), worker_id)
            .await
            .map_err(agent_db_error_to_app)?;
        let Some(run_row) = row else {
            return Ok(None);
        };
        let thread_row = agent::thread_by_id(self.executor(), run_row.thread_id)
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

    pub(crate) async fn renew_worker_lease(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<(), AppError> {
        agent::renew_run_lease(self.executor(), claimed.run.id, &claimed.lease_token)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn renew_worker_lease_request(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<(), AppError> {
        self.validate_worker_lease_request(request).await?;
        agent::renew_run_lease(self.executor(), request.run_id, &request.lease.fence_token)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn validate_worker_lease_request(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<agent::AgentRunRecord, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let run = agent::run_by_id(self.executor(), request.run_id)
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
        let thread = agent::thread_by_id(self.executor(), run.thread_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent thread 不存在: {}", run.thread_id)))?;
        Ok(AgentWorkerRun {
            run: run_snapshot(run),
            thread: thread_snapshot(thread)?,
            lease_token: request.lease.fence_token.clone(),
        })
    }
}

#[async_trait::async_trait]
impl AgentWorkerClaimApplicationPort for AgentApplication {
    async fn claim_worker_run(&self, worker_id: &str) -> Result<Option<AgentWorkerRun>, AppError> {
        AgentApplication::claim_worker_run(self, worker_id).await
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
