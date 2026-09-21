//! worker message load、approval 与 recovery。

use super::*;

impl AgentApplication {
    pub(crate) async fn load_message_request(
        &self,
        request: &AgentWorkerInboxMessageRequest,
    ) -> Result<AgentWorkerMessageSnapshotWire, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let claimed = self
            .worker_run_for_lease_request(&AgentWorkerRunLeaseRequest {
                run_id: request.run_id,
                lease: request.lease.clone(),
            })
            .await?;
        self.load_message_for_worker(claimed.run.thread_id, request.message_id)
            .await
            .map(Into::into)
    }

    pub(crate) async fn worker_history_request(
        &self,
        request: &AgentWorkerHistoryRequest,
    ) -> Result<AgentWorkerHistorySnapshotWire, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.worker_run_for_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        let user_messages = self.user_messages_for_run(request.run_id).await?;
        let latest_model_input_message_id =
            self.latest_model_input_message_id(request.run_id).await?;
        Ok(AgentWorkerHistorySnapshotWire {
            user_messages: user_messages.into_iter().map(Into::into).collect(),
            latest_model_input_message_id,
        })
    }

    /// Returns an approved operation for the given execution.  Approval
    /// consumption is intentionally left to the side-effect owner; an
    /// approved row is evidence, not permission to mutate unrelated data.
    pub(crate) async fn approved_execution(
        &self,
        thread_id: i64,
        execution_id: i64,
    ) -> Result<Option<approval::Model>, AppError> {
        Ok(agent::approvals_by_thread(self.connection(), thread_id)
            .await
            .map_err(agent_db_error_to_app)?
            .into_iter()
            .find(|row| row.execution_id == execution_id && row.status == "approved"))
    }

    pub(crate) async fn approved_execution_request(
        &self,
        request: &AgentWorkerApprovalCheckRequest,
    ) -> Result<AgentWorkerApprovalCheckSnapshotWire, AppError> {
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
        let approved = self
            .approved_execution(run.thread_id, request.execution_id)
            .await?
            .is_some();
        Ok(AgentWorkerApprovalCheckSnapshotWire { approved })
    }

    /// Atomically records a write approval request and releases the claimed
    /// Run into `waiting(approval)`. Runtime workers must use this façade
    /// instead of composing the two database operations themselves.
    pub(crate) async fn request_worker_approval(
        &self,
        claimed: &AgentWorkerRun,
        execution_id: i64,
        operation_json: Value,
        resource_preconditions_json: Option<Value>,
        expires_at: Option<chrono::NaiveDateTime>,
    ) -> Result<approval::Model, AppError> {
        self.request_worker_approval_fields(
            ApprovalRunFields {
                run_id: claimed.run.id,
                thread_id: claimed.run.thread_id,
                lease_token: &claimed.lease_token,
            },
            execution_id,
            operation_json,
            resource_preconditions_json,
            expires_at,
        )
        .await
    }

    pub(super) async fn request_worker_approval_fields(
        &self,
        fields: ApprovalRunFields<'_>,
        execution_id: i64,
        operation_json: Value,
        resource_preconditions_json: Option<Value>,
        expires_at: Option<chrono::NaiveDateTime>,
    ) -> Result<approval::Model, AppError> {
        let operation_digest = a4n_db::agent::sha256_hex(
            &serde_json::to_string(&operation_json).map_err(AppError::Serialization)?,
        );
        let (approval, _, _) = agent::request_approval_for_run_lease(
            self.connection(),
            fields.lease_token,
            a4n_domain::agent::AgentApprovalInput {
                thread_id: fields.thread_id,
                run_id: fields.run_id,
                execution_id,
                operation_digest,
                operation_json,
                resource_preconditions_json,
                expires_at,
            },
        )
        .await
        .map_err(agent_db_error_to_app)?;
        Ok(approval)
    }

    pub(crate) async fn request_worker_approval_request(
        &self,
        request: &AgentWorkerApprovalRequest,
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
        self.request_worker_approval_fields(
            ApprovalRunFields {
                run_id: run.id,
                thread_id: run.thread_id,
                lease_token: &request.lease.fence_token,
            },
            request.execution_id,
            request.operation_json.clone(),
            request.resource_preconditions_json.clone(),
            request.expires_at,
        )
        .await
        .map(|_| ())
    }

    pub(crate) async fn mark_recovery_required(
        &self,
        claimed: &AgentWorkerRun,
        reason: impl Into<String>,
    ) -> Result<run::Model, AppError> {
        agent::recovery_required_for_lease(
            self.connection(),
            claimed.run.id,
            &claimed.lease_token,
            Some(reason.into()),
        )
        .await
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn mark_recovery_required_request(
        &self,
        request: &AgentWorkerRecoveryRequest,
    ) -> Result<(), AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        agent::recovery_required_for_lease(
            self.connection(),
            request.run_id,
            &request.lease.fence_token,
            Some(request.reason.clone()),
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }
}
