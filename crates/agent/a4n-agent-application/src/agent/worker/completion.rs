//! worker run completion 与 failure。

use super::super::{AgentApplication, AgentWorkerRun};
use super::*;
use a4n_agent_runtime_contract::AgentWorkerCompletionApplicationPort;
use a4n_agent_runtime_contract::AgentWorkerFailureApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerFailureRequest;
use a4n_worker_contract::{
    AgentWorkerCompletionRequest, AgentWorkerCompletionSnapshotWire, AgentWorkerResultRefsRequest,
    AgentWorkerResultRefsSnapshotWire, AgentWorkerRunLeaseRequest,
    AgentWorkerRunStatusSnapshotWire,
};
use serde_json::Value;

impl AgentApplication {
    pub(crate) async fn complete_claimed_run(
        &self,
        claimed: &AgentWorkerRun,
        content: impl Into<String>,
        result_refs_json: Option<Value>,
        observed_input_message_id: i64,
    ) -> Result<bool, AppError> {
        self.complete_claimed_run_fields(
            RunLeaseFields {
                run_id: claimed.run.id,
                thread_id: claimed.run.thread_id,
                turn_id: claimed.run.turn_id,
                lease_token: &claimed.lease_token,
            },
            content.into(),
            result_refs_json,
            observed_input_message_id,
        )
        .await
    }

    pub(super) async fn complete_claimed_run_fields(
        &self,
        fields: RunLeaseFields<'_>,
        content: String,
        result_refs_json: Option<Value>,
        observed_input_message_id: i64,
    ) -> Result<bool, AppError> {
        let completed = agent::finish_run_with_message(
            self.connection(),
            fields.run_id,
            fields.lease_token,
            AgentMessageInput {
                thread_id: fields.thread_id,
                turn_id: Some(fields.turn_id),
                run_id: Some(fields.run_id),
                role: "assistant".into(),
                sender_kind: "agent".into(),
                sender_thread_id: None,
                client_message_id: Some(format!("agent-final:{}", fields.run_id)),
                content,
                status: "complete".into(),
                payload_json: None,
            },
            result_refs_json,
            observed_input_message_id,
        )
        .await
        .map_err(agent_db_error_to_app)?;
        Ok(completed.is_some())
    }

    pub(crate) async fn complete_worker_run_request(
        &self,
        request: &AgentWorkerCompletionRequest,
    ) -> Result<AgentWorkerCompletionSnapshotWire, AppError> {
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
        let completed = if run.parent_run_id.is_some() {
            self.complete_child_run_fields(
                RunLeaseFields {
                    run_id: run.id,
                    thread_id: run.thread_id,
                    turn_id: run.turn_id,
                    lease_token: &request.lease.fence_token,
                },
                request.content.clone(),
                request.result_summary.clone(),
                request.result_refs_json.clone(),
                request.observed_input_message_id,
            )
            .await?
        } else {
            self.complete_claimed_run_fields(
                RunLeaseFields {
                    run_id: run.id,
                    thread_id: run.thread_id,
                    turn_id: run.turn_id,
                    lease_token: &request.lease.fence_token,
                },
                request.content.clone(),
                request.result_refs_json.clone(),
                request.observed_input_message_id,
            )
            .await?
        };
        Ok(AgentWorkerCompletionSnapshotWire { completed })
    }

    pub(crate) async fn run_is_terminal_request(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<AgentWorkerRunStatusSnapshotWire, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(request).await?;
        let run = agent::run_by_id(self.connection(), request.run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent run 不存在: {}", request.run_id)))?;
        Ok(AgentWorkerRunStatusSnapshotWire {
            terminal: parse_run_status(&run.status)?.is_terminal(),
        })
    }

    pub(super) async fn fail_claimed_run(
        &self,
        claimed: &AgentWorkerRun,
        error_message: impl Into<String>,
    ) -> Result<run::Model, AppError> {
        let error_message = error_message.into();
        agent::fail_run_for_lease(
            self.connection(),
            claimed.run.id,
            &claimed.lease_token,
            Some(error_message),
        )
        .await
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn fail_child_run(
        &self,
        claimed: &AgentWorkerRun,
        error_message: impl Into<String>,
    ) -> Result<(), AppError> {
        if claimed.run.parent_run_id.is_none() {
            self.fail_claimed_run(claimed, error_message).await?;
            return Ok(());
        }
        agent::fail_child_delegation(
            self.connection(),
            a4n_db::agent::AgentChildFailureInput {
                child_run_id: claimed.run.id,
                child_lease_token: claimed.lease_token.clone(),
                error_message: error_message.into(),
            },
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn fail_worker_run(
        &self,
        request: &AgentWorkerFailureRequest,
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
        if run.parent_run_id.is_none() {
            agent::fail_run_for_lease(
                self.connection(),
                request.run_id,
                &request.lease.fence_token,
                Some(request.error_message.clone()),
            )
            .await
            .map(|_| ())
            .map_err(agent_db_error_to_app)
        } else {
            agent::fail_child_delegation(
                self.connection(),
                a4n_db::agent::AgentChildFailureInput {
                    child_run_id: request.run_id,
                    child_lease_token: request.lease.fence_token.clone(),
                    error_message: request.error_message.clone(),
                },
            )
            .await
            .map(|_| ())
            .map_err(agent_db_error_to_app)
        }
    }
}

#[async_trait::async_trait]
impl AgentWorkerCompletionApplicationPort for AgentApplication {
    async fn run_is_terminal(&self, claimed: &AgentWorkerRun) -> Result<bool, AppError> {
        AgentApplication::show_run(
            self,
            claimed.thread.owner_user_id,
            &claimed.run.id.to_string(),
        )
        .await
        .map(|run| run.status.is_terminal())
    }

    async fn result_refs_for_run(&self, claimed: &AgentWorkerRun) -> Result<Value, AppError> {
        AgentApplication::result_refs_for_run(self, claimed).await
    }

    async fn result_refs_for_run_request(
        &self,
        request: &AgentWorkerResultRefsRequest,
    ) -> Result<AgentWorkerResultRefsSnapshotWire, AppError> {
        AgentApplication::result_refs_for_run_request(self, request).await
    }

    async fn complete_worker_run_request(
        &self,
        request: &AgentWorkerCompletionRequest,
    ) -> Result<AgentWorkerCompletionSnapshotWire, AppError> {
        AgentApplication::complete_worker_run_request(self, request).await
    }

    async fn run_is_terminal_request(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<AgentWorkerRunStatusSnapshotWire, AppError> {
        AgentApplication::run_is_terminal_request(self, request).await
    }

    async fn complete_child_run(
        &self,
        claimed: &AgentWorkerRun,
        content: String,
        result_summary: String,
        result_refs_json: Option<Value>,
        observed_input_message_id: i64,
    ) -> Result<bool, AppError> {
        AgentApplication::complete_child_run(
            self,
            claimed,
            content,
            result_summary,
            result_refs_json,
            observed_input_message_id,
        )
        .await
    }

    async fn complete_claimed_run(
        &self,
        claimed: &AgentWorkerRun,
        content: String,
        result_refs_json: Option<Value>,
        observed_input_message_id: i64,
    ) -> Result<bool, AppError> {
        AgentApplication::complete_claimed_run(
            self,
            claimed,
            content,
            result_refs_json,
            observed_input_message_id,
        )
        .await
    }
}

#[async_trait::async_trait]
impl AgentWorkerFailureApplicationPort for AgentApplication {
    async fn fail_child_run(
        &self,
        claimed: &AgentWorkerRun,
        error_message: String,
    ) -> Result<(), AppError> {
        AgentApplication::fail_child_run(self, claimed, error_message).await
    }

    async fn fail_worker_run(&self, request: &AgentWorkerFailureRequest) -> Result<(), AppError> {
        AgentApplication::fail_worker_run(self, request).await
    }
}
