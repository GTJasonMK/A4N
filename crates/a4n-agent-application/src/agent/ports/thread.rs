//! Agent thread ports。

use super::*;

#[async_trait::async_trait]
impl a4n_api_contract::agent::AgentThreadMutationApplicationPort for AgentApplication {
    async fn ensure_main_thread(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::EnsureMainAgentThreadInput,
    ) -> Result<a4n_api_contract::agent::AgentThreadView, AppError> {
        AgentApplication::ensure_main_thread(self, user_id, input).await
    }

    async fn create_thread(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::CreateAgentThreadInput,
    ) -> Result<a4n_api_contract::agent::AgentThreadView, AppError> {
        AgentApplication::create_thread(self, user_id, input).await
    }

    async fn fork_thread(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::ForkAgentThreadInput,
    ) -> Result<a4n_api_contract::agent::AgentThreadView, AppError> {
        AgentApplication::fork_thread(self, user_id, input).await
    }

    async fn update_thread(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::UpdateAgentThreadInput,
    ) -> Result<a4n_api_contract::agent::AgentThreadView, AppError> {
        AgentApplication::update_thread(self, user_id, input).await
    }

    async fn archive_thread(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::ArchiveAgentThreadInput,
    ) -> Result<a4n_api_contract::agent::AgentThreadView, AppError> {
        AgentApplication::archive_thread(self, user_id, input).await
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::agent::AgentThreadReadApplicationPort for AgentApplication {
    async fn list_threads(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::ListAgentThreadsInput,
    ) -> Result<Vec<a4n_api_contract::agent::AgentThreadView>, AppError> {
        AgentApplication::list_threads(self, user_id, input).await
    }

    async fn show_thread(
        &self,
        user_id: i64,
        thread_id: &str,
    ) -> Result<a4n_api_contract::agent::AgentThreadSnapshot, AppError> {
        AgentApplication::show_thread(self, user_id, thread_id).await
    }

    async fn list_messages(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::ListAgentMessagesInput,
    ) -> Result<Vec<a4n_api_contract::agent::AgentMessageView>, AppError> {
        AgentApplication::list_messages(self, user_id, input).await
    }
}
