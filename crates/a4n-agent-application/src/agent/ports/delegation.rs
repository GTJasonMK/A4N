//! Agent delegation port。

use super::*;

#[async_trait::async_trait]
impl a4n_api_contract::agent::AgentDelegationApplicationPort for AgentApplication {
    async fn spawn_agent(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::SpawnAgentInput,
    ) -> Result<a4n_api_contract::agent::AgentDelegationView, AppError> {
        AgentApplication::spawn_agent(self, user_id, input).await
    }

    async fn review_delegation(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::ReviewAgentDelegationInput,
        parent_run_id: Option<i64>,
    ) -> Result<a4n_api_contract::agent::AgentDelegationView, AppError> {
        AgentApplication::review_delegation(self, user_id, input, parent_run_id).await
    }
}
