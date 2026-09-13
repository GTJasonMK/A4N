use super::{AgentApplication, AgentInboxSnapshot, AgentMessageSnapshot, AgentWorkerRun};
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentWorkerInboxClaimRequest, AgentWorkerInboxConsumeRequest, AgentWorkerInboxMessageRequest,
    AgentWorkerInboxPersistRequest, AgentWorkerInboxPersistSnapshotWire,
    AgentWorkerInboxSnapshotWire,
};

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

    async fn claim_inbox_request(
        &self,
        request: &AgentWorkerInboxClaimRequest,
    ) -> Result<Option<AgentWorkerInboxSnapshotWire>, AppError>;

    async fn consume_inbox_request(
        &self,
        request: &AgentWorkerInboxConsumeRequest,
    ) -> Result<AgentWorkerInboxSnapshotWire, AppError>;

    async fn persist_inbox_request(
        &self,
        request: &AgentWorkerInboxPersistRequest,
    ) -> Result<AgentWorkerInboxPersistSnapshotWire, AppError>;

    async fn load_message_request(
        &self,
        request: &AgentWorkerInboxMessageRequest,
    ) -> Result<a4n_worker_contract::AgentWorkerMessageSnapshotWire, AppError>;
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

    async fn claim_inbox_request(
        &self,
        request: &AgentWorkerInboxClaimRequest,
    ) -> Result<Option<AgentWorkerInboxSnapshotWire>, AppError> {
        AgentApplication::claim_inbox_request(self, request).await
    }

    async fn consume_inbox_request(
        &self,
        request: &AgentWorkerInboxConsumeRequest,
    ) -> Result<AgentWorkerInboxSnapshotWire, AppError> {
        AgentApplication::consume_inbox_request(self, request).await
    }

    async fn persist_inbox_request(
        &self,
        request: &AgentWorkerInboxPersistRequest,
    ) -> Result<AgentWorkerInboxPersistSnapshotWire, AppError> {
        AgentApplication::persist_inbox_request(self, request).await
    }

    async fn load_message_request(
        &self,
        request: &AgentWorkerInboxMessageRequest,
    ) -> Result<a4n_worker_contract::AgentWorkerMessageSnapshotWire, AppError> {
        AgentApplication::load_message_request(self, request).await
    }
}
