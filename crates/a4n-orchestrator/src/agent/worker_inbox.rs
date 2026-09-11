use super::{AgentApplication, AgentInboxSnapshot, AgentMessageSnapshot, AgentWorkerRun};
use a4n_error::AppError;

/// Runtime-facing boundary for claiming and materializing Agent inbox items.
#[async_trait::async_trait]
pub trait AgentWorkerInboxApplicationPort: Send + Sync {
    async fn claim_inbox_for_thread(
        &self,
        thread_id: i64,
    ) -> Result<Option<AgentInboxSnapshot>, AppError>;

    async fn consume_claimed_inbox(
        &self,
        row: &AgentInboxSnapshot,
    ) -> Result<AgentInboxSnapshot, AppError>;

    async fn persist_claimed_inbox(
        &self,
        claimed: &AgentWorkerRun,
        row: &AgentInboxSnapshot,
    ) -> Result<(AgentInboxSnapshot, AgentMessageSnapshot), AppError>;

    async fn load_message_for_worker(
        &self,
        thread_id: i64,
        message_id: i64,
    ) -> Result<AgentMessageSnapshot, AppError>;
}

#[async_trait::async_trait]
impl AgentWorkerInboxApplicationPort for AgentApplication {
    async fn claim_inbox_for_thread(
        &self,
        thread_id: i64,
    ) -> Result<Option<AgentInboxSnapshot>, AppError> {
        AgentApplication::claim_inbox_for_thread(self, thread_id).await
    }

    async fn consume_claimed_inbox(
        &self,
        row: &AgentInboxSnapshot,
    ) -> Result<AgentInboxSnapshot, AppError> {
        AgentApplication::consume_claimed_inbox(self, row).await
    }

    async fn persist_claimed_inbox(
        &self,
        claimed: &AgentWorkerRun,
        row: &AgentInboxSnapshot,
    ) -> Result<(AgentInboxSnapshot, AgentMessageSnapshot), AppError> {
        AgentApplication::persist_claimed_inbox(self, claimed, row).await
    }

    async fn load_message_for_worker(
        &self,
        thread_id: i64,
        message_id: i64,
    ) -> Result<AgentMessageSnapshot, AppError> {
        AgentApplication::load_message_for_worker(self, thread_id, message_id).await
    }
}
