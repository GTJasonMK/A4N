use super::AgentApplication;
use a4n_domain::agent::AgentWorkflowOrigin;
use a4n_error::AppError;

/// Runtime-facing boundary for binding a prepared Canvas Workflow to its
/// approved Agent start execution.
#[async_trait::async_trait]
pub trait AgentWorkflowBindingApplicationPort: Send + Sync {
    async fn bind_owned_workflow(
        &self,
        origin: &AgentWorkflowOrigin,
        workflow_id: i64,
    ) -> Result<(), AppError>;
}

#[async_trait::async_trait]
impl AgentWorkflowBindingApplicationPort for AgentApplication {
    async fn bind_owned_workflow(
        &self,
        origin: &AgentWorkflowOrigin,
        workflow_id: i64,
    ) -> Result<(), AppError> {
        AgentApplication::bind_owned_workflow(self, origin, workflow_id).await
    }
}
