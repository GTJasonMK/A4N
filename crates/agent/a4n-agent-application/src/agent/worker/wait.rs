//! worker wait、editor 与 durable update。

use super::super::{AgentApplication, AgentWorkerRun};
use super::*;
use crate::agent::agent_db_error_to_app;
use a4n_agent_runtime_contract::AgentEditorClaimApplicationPort;
use a4n_agent_runtime_contract::AgentEditorResponseApplicationPort;
use a4n_agent_runtime_contract::AgentEditorWaitApplicationPort;
use a4n_agent_runtime_contract::AgentWorkerActivationApplicationPort;
use a4n_agent_runtime_contract::AgentWorkerWaitApplicationPort;
use a4n_api_contract::agent::{AgentEditorRequest, RespondAgentEditorInput};
use a4n_db::agent;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerEditorWaitRequest;
use a4n_worker_contract::AgentWorkerRunLeaseRequest;
use a4n_worker_contract::AgentWorkerRunWaitRequest;

impl AgentApplication {
    pub(crate) async fn mark_turn_active(&self, claimed: &AgentWorkerRun) -> Result<(), AppError> {
        agent::mark_turn_active_for_run_lease(
            self.connection(),
            claimed.run.id,
            &claimed.lease_token,
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn wait_run(
        &self,
        claimed: &AgentWorkerRun,
        reason: &str,
    ) -> Result<(), AppError> {
        validate_waiting_reason(reason)?;
        agent::wait_run_for_lease(
            self.connection(),
            claimed.run.id,
            &claimed.lease_token,
            reason,
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn wait_run_request(
        &self,
        request: &AgentWorkerRunWaitRequest,
    ) -> Result<(), AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        validate_waiting_reason(&request.reason)?;
        agent::wait_run_for_lease(
            self.connection(),
            request.run_id,
            &request.lease.fence_token,
            &request.reason,
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn wait_for_editor(
        &self,
        claimed: &AgentWorkerRun,
        request: &a4n_api_contract::agent::AgentEditorRequest,
    ) -> Result<(), AppError> {
        let thread = agent::thread_by_id(self.connection(), claimed.thread.id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound("Agent thread 不存在".into()))?;
        let mut request = request.clone();
        request.source =
            canonicalize_editor_source(self.connection(), request.source, &thread).await?;
        agent::wait_run_for_lease_with_request(
            self.connection(),
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
        let run = agent::run_by_id(self.connection(), request.run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound("Agent Run 不存在".into()))?;
        let thread = agent::thread_by_id(self.connection(), run.thread_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound("Agent thread 不存在".into()))?;
        let mut editor_request = request.editor_request.clone();
        editor_request.source =
            canonicalize_editor_source(self.connection(), editor_request.source, &thread).await?;
        agent::wait_run_for_lease_with_request(
            self.connection(),
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
        input: &a4n_api_contract::agent::RespondAgentEditorInput,
    ) -> Result<(AgentWorkerRun, a4n_api_contract::agent::AgentEditorRequest), AppError> {
        let run_id = parse_id(&input.run_id, "run_id")?;
        let execution_id = parse_id(&input.execution_id, "execution_id")?;
        let run = agent::run_by_id(self.connection(), run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound("Agent Run 不存在".into()))?;
        let thread = self.owned_thread(user_id, run.thread_id).await?;
        let payload = agent::latest_event_by_run_and_kind(self.connection(), run_id, "run_waiting")
            .await
            .map_err(agent_db_error_to_app)?
            .and_then(|event| event.payload_json)
            .and_then(|payload| payload.get("editor_request").cloned())
            .ok_or_else(|| AppError::InvalidArgument("Run 没有待响应的编辑器请求".into()))?;
        let mut request: a4n_api_contract::agent::AgentEditorRequest =
            from_json(payload, "editor_request")?;
        if request.execution_id != input.execution_id
            || request.source.view_instance_id != input.view_instance_id
        {
            return Err(AppError::Forbidden("响应与原始视图或执行不匹配".into()));
        }
        request.source =
            canonicalize_editor_source(self.connection(), request.source, &thread).await?;
        let run = agent::claim_editor_execution(self.connection(), user_id, run_id, execution_id)
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
            self.connection(),
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

    pub(crate) async fn append_worker_update(
        &self,
        claimed: &AgentWorkerRun,
        kind: &str,
        status: &str,
        public_text: impl Into<String>,
        payload_json: Option<Value>,
    ) -> Result<event::Model, AppError> {
        self.append_worker_update_fields(
            RunLeaseFields {
                run_id: claimed.run.id,
                thread_id: claimed.run.thread_id,
                turn_id: claimed.run.turn_id,
                lease_token: &claimed.lease_token,
            },
            kind.to_string(),
            status.to_string(),
            public_text.into(),
            payload_json,
        )
        .await
    }

    pub(super) async fn append_worker_update_fields(
        &self,
        fields: RunLeaseFields<'_>,
        kind: String,
        status: String,
        public_text: String,
        payload_json: Option<Value>,
    ) -> Result<event::Model, AppError> {
        agent::append_event_for_run_lease(
            self.connection(),
            fields.run_id,
            fields.lease_token,
            AgentEventInput {
                thread_id: fields.thread_id,
                turn_id: Some(fields.turn_id),
                run_id: Some(fields.run_id),
                execution_id: None,
                delegation_id: None,
                message_id: None,
                actor_kind: "agent".into(),
                actor_id: Some(fields.run_id.to_string()),
                kind,
                status,
                public_text,
                payload_json,
            },
        )
        .await
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn append_worker_update_request(
        &self,
        request: &AgentWorkerEventRequest,
    ) -> Result<(), AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        let run = agent::run_by_id(self.connection(), request.run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent run 不存在: {}", request.run_id)))?;
        self.append_worker_update_fields(
            RunLeaseFields {
                run_id: run.id,
                thread_id: run.thread_id,
                turn_id: run.turn_id,
                lease_token: &request.lease.fence_token,
            },
            request.kind.clone(),
            request.status.clone(),
            request.public_text.clone(),
            request.payload_json.clone(),
        )
        .await
        .map(|_| ())
    }
}

#[async_trait::async_trait]
impl AgentWorkerActivationApplicationPort for AgentApplication {
    async fn mark_turn_active(&self, claimed: &AgentWorkerRun) -> Result<(), AppError> {
        AgentApplication::mark_turn_active(self, claimed).await
    }

    async fn mark_turn_active_request(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<(), AppError> {
        AgentApplication::mark_turn_active_request(self, request).await
    }
}

#[async_trait::async_trait]
impl AgentWorkerWaitApplicationPort for AgentApplication {
    async fn wait_run(&self, claimed: &AgentWorkerRun, reason: &str) -> Result<(), AppError> {
        AgentApplication::wait_run(self, claimed, reason).await
    }

    async fn wait_run_request(&self, request: &AgentWorkerRunWaitRequest) -> Result<(), AppError> {
        AgentApplication::wait_run_request(self, request).await
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

impl AgentApplication {
    pub async fn expire_editor_requests(&self, limit: u64) -> Result<usize, AppError> {
        agent::expire_editor_requests(self.connection(), limit)
            .await
            .map_err(agent_db_error_to_app)
    }
}

#[async_trait::async_trait]
impl AgentEditorWaitApplicationPort for AgentApplication {
    async fn expire_editor_requests(&self, limit: u64) -> Result<usize, AppError> {
        AgentApplication::expire_editor_requests(self, limit).await
    }
}
