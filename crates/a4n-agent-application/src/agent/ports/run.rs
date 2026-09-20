//! Agent run ports。

use super::*;

#[async_trait::async_trait]
impl a4n_api_contract::agent::AgentRunMutationApplicationPort for AgentApplication {
    async fn send_message(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::SendAgentMessageInput,
        runtime_snapshot_required: bool,
        require_runtime_snapshot_for_new_run: bool,
    ) -> Result<a4n_api_contract::agent::AgentMessageAccepted, AppError> {
        AgentApplication::send_message(
            self,
            user_id,
            input,
            runtime_snapshot_required,
            require_runtime_snapshot_for_new_run,
        )
        .await
    }

    async fn control_run(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::ControlAgentRunInput,
    ) -> Result<a4n_api_contract::agent::AgentRunView, AppError> {
        AgentApplication::control_run(self, user_id, input).await
    }

    async fn send_message_to_child(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::SendAgentMessageToChildInput,
        runtime_snapshot_required: bool,
        require_runtime_snapshot_for_new_run: bool,
    ) -> Result<a4n_api_contract::agent::AgentMessageAccepted, AppError> {
        AgentApplication::send_message_to_child(
            self,
            user_id,
            input,
            runtime_snapshot_required,
            require_runtime_snapshot_for_new_run,
        )
        .await
    }

    async fn control_child_agent(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::ControlChildAgentInput,
    ) -> Result<a4n_api_contract::agent::AgentRunView, AppError> {
        AgentApplication::control_child_agent(self, user_id, input).await
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::agent::AgentRunReadApplicationPort for AgentApplication {
    async fn show_run(
        &self,
        user_id: i64,
        run_id: &str,
    ) -> Result<a4n_api_contract::agent::AgentRunView, AppError> {
        AgentApplication::show_run(self, user_id, run_id).await
    }
}
