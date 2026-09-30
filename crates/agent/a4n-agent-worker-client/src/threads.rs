use super::*;

#[async_trait::async_trait]
impl AgentThreadMutationApplicationPort for AgentWorkerClient {
    async fn ensure_main_thread(
        &self,
        user_id: i64,
        input: EnsureMainAgentThreadInput,
    ) -> Result<AgentThreadView, AppError> {
        self.command(user_id, AGENT_THREAD_ENSURE_MAIN_OPERATION, input)
            .await
    }

    async fn create_thread(
        &self,
        user_id: i64,
        input: CreateAgentThreadInput,
    ) -> Result<AgentThreadView, AppError> {
        self.command(user_id, AGENT_THREAD_CREATE_OPERATION, input)
            .await
    }

    async fn fork_thread(
        &self,
        user_id: i64,
        input: ForkAgentThreadInput,
    ) -> Result<AgentThreadView, AppError> {
        self.command(user_id, AGENT_THREAD_FORK_OPERATION, input)
            .await
    }

    async fn update_thread(
        &self,
        user_id: i64,
        input: UpdateAgentThreadInput,
    ) -> Result<AgentThreadView, AppError> {
        self.command(user_id, AGENT_THREAD_UPDATE_OPERATION, input)
            .await
    }

    async fn archive_thread(
        &self,
        user_id: i64,
        input: ArchiveAgentThreadInput,
    ) -> Result<AgentThreadView, AppError> {
        self.command(user_id, AGENT_THREAD_ARCHIVE_OPERATION, input)
            .await
    }
}

#[async_trait::async_trait]
impl AgentThreadReadApplicationPort for AgentWorkerClient {
    async fn list_threads(
        &self,
        user_id: i64,
        input: ListAgentThreadsInput,
    ) -> Result<Vec<AgentThreadView>, AppError> {
        self.command(user_id, AGENT_THREAD_LIST_OPERATION, input)
            .await
    }

    async fn show_thread(
        &self,
        user_id: i64,
        thread_id: &str,
    ) -> Result<AgentThreadSnapshot, AppError> {
        self.command(user_id, AGENT_THREAD_SHOW_OPERATION, thread_id.to_string())
            .await
    }

    async fn list_messages(
        &self,
        user_id: i64,
        input: ListAgentMessagesInput,
    ) -> Result<Vec<AgentMessageView>, AppError> {
        self.command(user_id, AGENT_THREAD_MESSAGES_OPERATION, input)
            .await
    }
}
