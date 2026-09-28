use super::*;

#[async_trait::async_trait]
impl AgentDelegationApplicationPort for AgentWorkerClient {
    async fn spawn_agent(
        &self,
        user_id: i64,
        input: SpawnAgentInput,
    ) -> Result<AgentDelegationView, AppError> {
        self.command(user_id, AGENT_DELEGATION_SPAWN_OPERATION, input)
            .await
    }

    async fn review_delegation(
        &self,
        user_id: i64,
        input: ReviewAgentDelegationInput,
        parent_run_id: Option<i64>,
    ) -> Result<AgentDelegationView, AppError> {
        self.command(
            user_id,
            AGENT_DELEGATION_REVIEW_OPERATION,
            AgentDelegationReviewCommand {
                input,
                parent_run_id,
            },
        )
        .await
    }
}
