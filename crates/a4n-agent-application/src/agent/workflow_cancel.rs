use super::AgentApplication;
use a4n_agent_runtime_contract::{AgentWorkflowCancelApplicationPort, AgentWorkflowCancelSnapshot};
use a4n_error::AppError;

impl AgentApplication {
    pub async fn workflow_cancel_candidates(
        &self,
        after_id: i64,
    ) -> Result<Vec<AgentWorkflowCancelSnapshot>, AppError> {
        let workflows = self.active_owned_workflows(after_id).await?;
        let mut candidates = Vec::with_capacity(workflows.len());
        for workflow in workflows {
            let owner = self
                .workflow_owner(workflow.id)
                .await?
                .ok_or_else(|| AppError::InvalidArgument("Workflow Agent owner 丢失".into()))?;
            candidates.push(AgentWorkflowCancelSnapshot {
                workflow_run_id: workflow.id,
                owner_stopped: self.generation_owner_stopped(&owner).await?,
            });
        }
        Ok(candidates)
    }
}

#[async_trait::async_trait]
impl AgentWorkflowCancelApplicationPort for AgentApplication {
    async fn workflow_cancel_candidates(
        &self,
        after_id: i64,
    ) -> Result<Vec<AgentWorkflowCancelSnapshot>, AppError> {
        AgentApplication::workflow_cancel_candidates(self, after_id).await
    }
}
