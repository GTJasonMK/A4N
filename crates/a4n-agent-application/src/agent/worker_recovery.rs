use super::{AgentApplication, AgentWorkerRun};
use a4n_error::AppError;

/// Runtime-facing boundary for marking a leased Run as requiring recovery.
///
/// Lease fencing and the durable recovery transition remain in the Agent
/// owner; Runtime only supplies the claimed Run and reason.
#[async_trait::async_trait]
pub trait AgentWorkerRecoveryApplicationPort: Send + Sync {
    async fn mark_recovery_required(
        &self,
        claimed: &AgentWorkerRun,
        reason: String,
    ) -> Result<(), AppError>;
}

#[async_trait::async_trait]
impl AgentWorkerRecoveryApplicationPort for AgentApplication {
    async fn mark_recovery_required(
        &self,
        claimed: &AgentWorkerRun,
        reason: String,
    ) -> Result<(), AppError> {
        AgentApplication::mark_recovery_required(self, claimed, reason)
            .await
            .map(|_| ())
    }
}
