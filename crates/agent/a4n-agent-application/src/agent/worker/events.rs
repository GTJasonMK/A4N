use super::super::{AgentApplication, AgentWorkerRun};
use super::*;
use a4n_agent_runtime_contract::AgentWorkerEventWriteApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerEventRequest;
use a4n_worker_contract::AgentWorkerRunLeaseRequest;
use serde_json::Value;

impl AgentApplication {
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
impl AgentWorkerEventWriteApplicationPort for AgentApplication {
    async fn append_worker_update_request(
        &self,
        request: &AgentWorkerEventRequest,
    ) -> Result<(), AppError> {
        AgentApplication::append_worker_update_request(self, request).await
    }

    async fn append_worker_update(
        &self,
        claimed: &AgentWorkerRun,
        kind: &str,
        status: &str,
        public_text: String,
        payload_json: Option<Value>,
    ) -> Result<(), AppError> {
        AgentApplication::append_worker_update(
            self,
            claimed,
            kind,
            status,
            public_text,
            payload_json,
        )
        .await
        .map(|_| ())
    }
}
