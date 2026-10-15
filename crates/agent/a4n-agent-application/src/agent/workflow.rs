use super::AgentApplication;
use crate::agent::agent_db_error_to_app;
use a4n_agent_runtime_contract::AgentWorkflowBindingApplicationPort;
use a4n_agent_runtime_contract::{AgentWorkflowCancelApplicationPort, AgentWorkflowCancelSnapshot};
use a4n_domain::agent::AgentWorkflowOrigin;
use a4n_error::AppError;

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

impl AgentApplication {
    pub async fn workflow_cancel_candidates(
        &self,
        after_id: i64,
    ) -> Result<Vec<AgentWorkflowCancelSnapshot>, AppError> {
        let workflows = self.active_owned_workflows(after_id).await?;
        let mut candidates = Vec::with_capacity(workflows.len());
        let mut owners = Vec::with_capacity(workflows.len());
        for workflow in workflows {
            let owner = self
                .workflow_owner(workflow)
                .await?
                .ok_or_else(|| AppError::InvalidArgument("Workflow Agent owner 丢失".into()))?;
            candidates.push(AgentWorkflowCancelSnapshot {
                workflow_run_id: workflow,
                owner_stopped: false,
            });
            owners.push(owner);
        }

        let owner_statuses = a4n_db::agent::generation_owner_statuses(self.executor(), &owners)
            .await
            .map_err(agent_db_error_to_app)?;
        let mut owners_for_budget = Vec::with_capacity(candidates.len());
        for (index, owner) in owners.iter().enumerate() {
            let stopped_by_status = *owner_statuses.get(&owner.id).ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!(
                    "Workflow owner status batch omitted run {}",
                    owner.id
                ))
            })?;
            candidates[index].owner_stopped = stopped_by_status;
            if !stopped_by_status {
                owners_for_budget.push((index, owner.id));
            }
        }

        let owner_ids = owners_for_budget
            .iter()
            .map(|(_, owner_id)| *owner_id)
            .collect::<Vec<_>>();
        if owner_ids.is_empty() {
            return Ok(candidates);
        }
        let budgets = a4n_db::agent::run_budget_snapshots(self.executor(), &owner_ids)
            .await
            .map_err(agent_db_error_to_app)?;
        for (index, owner_id) in owners_for_budget {
            let budget = budgets.get(&owner_id).ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!(
                    "Workflow owner budget batch omitted run {owner_id}"
                ))
            })?;
            candidates[index].owner_stopped = budget.remaining.max_duration_seconds == Some(0);
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
