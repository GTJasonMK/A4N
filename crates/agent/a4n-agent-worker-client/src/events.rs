use super::*;

#[async_trait::async_trait]
impl AgentEventReadApplicationPort for AgentWorkerClient {
    async fn events(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::AgentEventsInput,
    ) -> Result<Vec<AgentEvent>, AppError> {
        self.command(user_id, AGENT_EVENT_LIST_OPERATION, input)
            .await
    }
}
