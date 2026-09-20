//! Agent event read port。

use super::*;

#[async_trait::async_trait]
impl a4n_api_contract::agent::AgentEventReadApplicationPort for AgentApplication {
    async fn events(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::AgentEventsInput,
    ) -> Result<Vec<a4n_api_contract::agent::AgentEvent>, AppError> {
        AgentApplication::events(self, user_id, input).await
    }
}
