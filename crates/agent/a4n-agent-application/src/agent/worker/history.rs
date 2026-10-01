use super::super::{AgentApplication, AgentMessageSnapshot};
use super::*;
use a4n_agent_runtime_contract::AgentWorkerHistoryApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::{AgentWorkerHistoryRequest, AgentWorkerHistorySnapshotWire};

impl AgentApplication {
    pub async fn user_messages_for_run(
        &self,
        run_id: i64,
    ) -> Result<Vec<AgentMessageSnapshot>, AppError> {
        Ok(agent::messages_by_run(self.connection(), run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .into_iter()
            .filter(|message| message.role == "user")
            .map(message_snapshot)
            .collect())
    }

    pub async fn latest_model_input_message_id(
        &self,
        run_id: i64,
    ) -> Result<Option<i64>, AppError> {
        let executions = agent::executions_by_runs(self.connection(), vec![run_id])
            .await
            .map_err(agent_db_error_to_app)?;
        Ok(executions
            .iter()
            .filter(|execution| execution.execution_kind == "model_call")
            .filter(|execution| execution.status == "succeeded")
            .filter_map(|execution| {
                execution
                    .resource_preconditions_json
                    .as_ref()?
                    .get("input_message_id")?
                    .as_i64()
            })
            .max())
    }

    pub(crate) async fn worker_history_request(
        &self,
        request: &AgentWorkerHistoryRequest,
    ) -> Result<AgentWorkerHistorySnapshotWire, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let claimed = self
            .worker_run_for_lease_request(&AgentWorkerRunLeaseRequest {
                run_id: request.run_id,
                lease: request.lease.clone(),
            })
            .await?;
        let page = if let Some(page) = &request.page {
            Some(
                self.list_messages(
                    claimed.thread.owner_user_id,
                    a4n_api_contract::agent::ListAgentMessagesInput {
                        thread_id: claimed.run.thread_id.to_string(),
                        before: page.before.clone(),
                        limit: Some(page.limit.unwrap_or(20)),
                    },
                )
                .await?,
            )
        } else {
            None
        };
        let user_messages = self.user_messages_for_run(request.run_id).await?;
        let latest_model_input_message_id =
            self.latest_model_input_message_id(request.run_id).await?;
        Ok(AgentWorkerHistorySnapshotWire {
            user_messages: user_messages.into_iter().map(Into::into).collect(),
            latest_model_input_message_id,
            page,
        })
    }
}

#[async_trait::async_trait]
impl AgentWorkerHistoryApplicationPort for AgentApplication {
    async fn user_messages_for_run(
        &self,
        run_id: i64,
    ) -> Result<Vec<AgentMessageSnapshot>, AppError> {
        AgentApplication::user_messages_for_run(self, run_id).await
    }

    async fn latest_model_input_message_id(&self, run_id: i64) -> Result<Option<i64>, AppError> {
        AgentApplication::latest_model_input_message_id(self, run_id).await
    }

    async fn worker_history_request(
        &self,
        request: &AgentWorkerHistoryRequest,
    ) -> Result<AgentWorkerHistorySnapshotWire, AppError> {
        AgentApplication::worker_history_request(self, request).await
    }
}
