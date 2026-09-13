use super::{AgentApplication, AgentWorkerRun};
use a4n_agent_runtime_contract::AgentWorkerCompletionApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentWorkerCompletionRequest, AgentWorkerCompletionSnapshotWire, AgentWorkerResultRefsRequest,
    AgentWorkerResultRefsSnapshotWire, AgentWorkerRunLeaseRequest,
    AgentWorkerRunStatusSnapshotWire,
};
use serde_json::Value;

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
