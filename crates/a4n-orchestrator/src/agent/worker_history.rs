use super::{AgentApplication, AgentMessageSnapshot};
use a4n_error::AppError;

/// Runtime-facing boundary for the message history facts used by optimization
/// continuation.
#[async_trait::async_trait]
pub trait AgentWorkerHistoryApplicationPort: Send + Sync {
    async fn user_messages_for_run(
        &self,
        run_id: i64,
    ) -> Result<Vec<AgentMessageSnapshot>, AppError>;

    async fn latest_model_input_message_id(&self, run_id: i64) -> Result<Option<i64>, AppError>;
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
}
