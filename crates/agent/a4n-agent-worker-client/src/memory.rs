use super::*;

#[async_trait::async_trait]
impl AgentMemoryReadApplicationPort for AgentWorkerClient {
    async fn list_memories(
        &self,
        user_id: i64,
        scope_kind: Option<AgentMemoryScopeKind>,
        scope_id: Option<String>,
    ) -> Result<Vec<AgentMemoryView>, AppError> {
        self.command(
            user_id,
            AGENT_MEMORY_LIST_OPERATION,
            AgentMemoryListInput {
                scope_kind,
                scope_id,
            },
        )
        .await
    }
}

#[async_trait::async_trait]
impl AgentMemoryMutationApplicationPort for AgentWorkerClient {
    async fn create_memory(
        &self,
        user_id: i64,
        input: AgentMemoryInput,
    ) -> Result<AgentMemoryView, AppError> {
        self.command(user_id, AGENT_MEMORY_CREATE_OPERATION, input)
            .await
    }

    async fn retract_memory(
        &self,
        user_id: i64,
        input: RetractAgentMemoryInput,
    ) -> Result<AgentMemoryView, AppError> {
        self.command(user_id, AGENT_MEMORY_RETRACT_OPERATION, input)
            .await
    }
}
