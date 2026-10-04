//! Editor wait, response claim, and requeue operations.

use super::super::{AgentApplication, AgentWorkerRun};
use super::*;
use crate::agent::agent_db_error_to_app;
use a4n_agent_runtime_contract::{
    AgentEditorClaimApplicationPort, AgentEditorResponseApplicationPort,
    AgentEditorWaitApplicationPort,
};
use a4n_api_contract::agent::{AgentEditorRequest, RespondAgentEditorInput};
use a4n_db::agent;
use a4n_error::AppError;
use a4n_worker_contract::{AgentWorkerEditorWaitRequest, AgentWorkerRunLeaseRequest};

impl AgentApplication {
    pub(crate) async fn wait_for_editor(
        &self,
        claimed: &AgentWorkerRun,
        request: &AgentEditorRequest,
    ) -> Result<(), AppError> {
        let thread = agent::thread_by_id(self.executor(), claimed.thread.id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound("Agent thread 不存在".into()))?;
        let mut request = request.clone();
        request.source =
            canonicalize_editor_source(self.executor(), request.source, &thread).await?;
        agent::wait_run_for_lease_with_request(
            self.executor(),
            claimed.run.id,
            &claimed.lease_token,
            "external",
            Some(serde_json::to_value(request)?),
        )
        .await
        .map_err(agent_db_error_to_app)?;
        Ok(())
    }

    pub(crate) async fn wait_for_editor_request(
        &self,
        request: &AgentWorkerEditorWaitRequest,
    ) -> Result<(), AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        let run = agent::run_by_id(self.executor(), request.run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound("Agent Run 不存在".into()))?;
        let thread = agent::thread_by_id(self.executor(), run.thread_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound("Agent thread 不存在".into()))?;
        let mut editor_request = request.editor_request.clone();
        editor_request.source =
            canonicalize_editor_source(self.executor(), editor_request.source, &thread).await?;
        agent::wait_run_for_lease_with_request(
            self.executor(),
            request.run_id,
            &request.lease.fence_token,
            "external",
            Some(serde_json::to_value(&editor_request)?),
        )
        .await
        .map_err(agent_db_error_to_app)?;
        Ok(())
    }

    pub(crate) async fn claim_editor_request(
        &self,
        user_id: i64,
        input: &RespondAgentEditorInput,
    ) -> Result<(AgentWorkerRun, AgentEditorRequest), AppError> {
        let run_id = parse_id(&input.run_id, "run_id")?;
        let execution_id = parse_id(&input.execution_id, "execution_id")?;
        let run = agent::run_by_id(self.executor(), run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound("Agent Run 不存在".into()))?;
        let thread = self.owned_thread(user_id, run.thread_id).await?;
        let payload = agent::latest_event_by_run_and_kind(self.executor(), run_id, "run_waiting")
            .await
            .map_err(agent_db_error_to_app)?
            .and_then(|event| event.payload_json)
            .and_then(|payload| payload.get("editor_request").cloned())
            .ok_or_else(|| AppError::InvalidArgument("Run 没有待响应的编辑器请求".into()))?;
        let mut request: AgentEditorRequest = from_json(payload, "editor_request")?;
        if request.execution_id != input.execution_id
            || request.source.view_instance_id != input.view_instance_id
        {
            return Err(AppError::Forbidden("响应与原始视图或执行不匹配".into()));
        }
        request.source =
            canonicalize_editor_source(self.executor(), request.source, &thread).await?;
        let run = agent::claim_editor_execution(self.executor(), user_id, run_id, execution_id)
            .await
            .map_err(agent_db_error_to_app)?;
        let lease_token = run
            .lease_token
            .clone()
            .ok_or_else(|| AppError::InvalidState {
                current: "编辑器响应缺少执行租约".into(),
                operation: "领取编辑器请求".into(),
            })?;
        Ok((
            AgentWorkerRun {
                run: run_snapshot(run),
                thread: thread_snapshot(thread)?,
                lease_token,
            },
            request,
        ))
    }

    pub(crate) async fn requeue_after_external_response(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<(), AppError> {
        agent::update_run_state(
            self.executor(),
            claimed.run.id,
            Some(&claimed.lease_token),
            "queued",
            None,
            None,
        )
        .await
        .map_err(agent_db_error_to_app)?;
        Ok(())
    }

    pub async fn expire_editor_requests(&self, limit: u64) -> Result<usize, AppError> {
        agent::expire_editor_requests(self.executor(), limit)
            .await
            .map_err(agent_db_error_to_app)
    }
}

#[async_trait::async_trait]
impl AgentEditorClaimApplicationPort for AgentApplication {
    async fn claim_editor_request(
        &self,
        user_id: i64,
        input: &RespondAgentEditorInput,
    ) -> Result<(AgentWorkerRun, AgentEditorRequest), AppError> {
        AgentApplication::claim_editor_request(self, user_id, input).await
    }
}

#[async_trait::async_trait]
impl AgentEditorResponseApplicationPort for AgentApplication {
    async fn wait_for_editor(
        &self,
        claimed: &AgentWorkerRun,
        request: &AgentEditorRequest,
    ) -> Result<(), AppError> {
        AgentApplication::wait_for_editor(self, claimed, request).await
    }

    async fn wait_for_editor_request(
        &self,
        request: &AgentWorkerEditorWaitRequest,
    ) -> Result<(), AppError> {
        AgentApplication::wait_for_editor_request(self, request).await
    }

    async fn requeue_after_external_response(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<(), AppError> {
        AgentApplication::requeue_after_external_response(self, claimed).await
    }
}

#[async_trait::async_trait]
impl AgentEditorWaitApplicationPort for AgentApplication {
    async fn expire_editor_requests(&self, limit: u64) -> Result<usize, AppError> {
        AgentApplication::expire_editor_requests(self, limit).await
    }
}
