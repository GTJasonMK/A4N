use super::{AgentApplication, AgentWorkerRun};
use a4n_error::AppError;
use serde_json::Value;

/// Runtime-facing boundary for terminal completion of a claimed Agent Run.
#[async_trait::async_trait]
pub trait AgentWorkerCompletionApplicationPort: Send + Sync {
    async fn result_refs_for_run(&self, claimed: &AgentWorkerRun) -> Result<Value, AppError>;

    async fn complete_child_run(
        &self,
        claimed: &AgentWorkerRun,
        content: String,
        result_summary: String,
        result_refs_json: Option<Value>,
        observed_input_message_id: i64,
    ) -> Result<bool, AppError>;

    async fn complete_claimed_run(
        &self,
        claimed: &AgentWorkerRun,
        content: String,
        result_refs_json: Option<Value>,
        observed_input_message_id: i64,
    ) -> Result<bool, AppError>;
}

#[async_trait::async_trait]
impl AgentWorkerCompletionApplicationPort for AgentApplication {
    async fn result_refs_for_run(&self, claimed: &AgentWorkerRun) -> Result<Value, AppError> {
        AgentApplication::result_refs_for_run(self, claimed).await
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
