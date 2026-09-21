use super::AgentApplication;
use crate::agent::agent_db_error_to_app;
use a4n_agent_runtime_contract::AgentEditorWaitApplicationPort;
use a4n_db::agent;
use a4n_error::AppError;

impl AgentApplication {
    pub async fn expire_editor_requests(&self, limit: u64) -> Result<usize, AppError> {
        agent::expire_editor_requests(self.connection(), limit)
            .await
            .map_err(agent_db_error_to_app)
    }
}

#[async_trait::async_trait]
impl AgentEditorWaitApplicationPort for AgentApplication {
    async fn expire_editor_requests(&self, limit: u64) -> Result<usize, AppError> {
        AgentApplication::expire_editor_requests(self, limit).await
    }
}
