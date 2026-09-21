//! Agent memory ports。

use super::*;

#[async_trait::async_trait]
impl a4n_api_contract::agent::AgentMemoryReadApplicationPort for AgentApplication {
    async fn list_memories(
        &self,
        user_id: i64,
        scope_kind: Option<a4n_api_contract::agent::AgentMemoryScopeKind>,
        scope_id: Option<String>,
    ) -> Result<Vec<a4n_api_contract::agent::AgentMemoryView>, AppError> {
        AgentApplication::list_memories(self, user_id, scope_kind, scope_id).await
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::agent::AgentMemoryMutationApplicationPort for AgentApplication {
    async fn create_memory(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::AgentMemoryInput,
    ) -> Result<a4n_api_contract::agent::AgentMemoryView, AppError> {
        AgentApplication::create_memory(self, user_id, input).await
    }

    async fn retract_memory(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::RetractAgentMemoryInput,
    ) -> Result<a4n_api_contract::agent::AgentMemoryView, AppError> {
        AgentApplication::retract_memory(self, user_id, input).await
    }
}
