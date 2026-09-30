use super::*;

#[async_trait::async_trait]
impl AgentApprovalApplicationPort for AgentWorkerClient {
    async fn resolve_approval(
        &self,
        user_id: i64,
        input: ResolveAgentApprovalInput,
    ) -> Result<AgentApprovalView, AppError> {
        self.command(user_id, AGENT_APPROVAL_RESOLVE_OPERATION, input)
            .await
    }
}
