use super::AgentApplication;
use a4n_agent_runtime_contract::{
    AgentGenerationCancelApplicationPort, AgentGenerationCancelSnapshot,
};
use a4n_error::AppError;

impl AgentApplication {
    pub async fn generation_cancel_candidates(
        &self,
        after_task_id: &str,
    ) -> Result<Vec<AgentGenerationCancelSnapshot>, AppError> {
        let tasks = self.active_owned_generation_tasks(after_task_id).await?;
        let mut candidates = Vec::with_capacity(tasks.len());
        for task in tasks {
            let attempt = u32::try_from(task.attempt)
                .map_err(|_| AppError::InvalidArgument("Generation attempt 无效".into()))?;
            let owner = self
                .generation_owner(&task.task_id, attempt)
                .await?
                .ok_or_else(|| AppError::InvalidArgument("Generation owner 丢失".into()))?;
            candidates.push(AgentGenerationCancelSnapshot {
                task_id: task.task_id,
                attempt,
                requested_by_user_id: task.requested_by_user_id,
                status: task.status,
                owner_stopped: self.generation_owner_stopped(&owner).await?,
            });
        }
        Ok(candidates)
    }
}

#[async_trait::async_trait]
impl AgentGenerationCancelApplicationPort for AgentApplication {
    async fn generation_cancel_candidates(
        &self,
        after_task_id: &str,
    ) -> Result<Vec<AgentGenerationCancelSnapshot>, AppError> {
        AgentApplication::generation_cancel_candidates(self, after_task_id).await
    }
}
