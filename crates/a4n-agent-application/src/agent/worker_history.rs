use super::{AgentApplication, AgentMessageSnapshot};
use a4n_agent_runtime_contract::AgentWorkerHistoryApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::{AgentWorkerHistoryRequest, AgentWorkerHistorySnapshotWire};

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
