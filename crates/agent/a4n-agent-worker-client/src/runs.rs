use super::*;

#[async_trait::async_trait]
impl AgentRunReadApplicationPort for AgentWorkerClient {
    async fn show_run(&self, user_id: i64, run_id: &str) -> Result<AgentRunView, AppError> {
        self.command(user_id, AGENT_RUN_SHOW_OPERATION, run_id.to_string())
            .await
    }
}

#[async_trait::async_trait]
impl AgentRunMutationApplicationPort for AgentWorkerClient {
    async fn send_message(
        &self,
        user_id: i64,
        input: SendAgentMessageInput,
        runtime_snapshot_required: bool,
        require_runtime_snapshot_for_new_run: bool,
    ) -> Result<AgentMessageAccepted, AppError> {
        self.command(
            user_id,
            AGENT_RUN_SEND_OPERATION,
            AgentRunRuntimeCommand {
                input,
                runtime_snapshot_required,
                require_runtime_snapshot_for_new_run,
            },
        )
        .await
    }

    async fn control_run(
        &self,
        user_id: i64,
        input: ControlAgentRunInput,
    ) -> Result<AgentRunView, AppError> {
        self.command(user_id, AGENT_RUN_CONTROL_OPERATION, input)
            .await
    }

    async fn send_message_to_child(
        &self,
        user_id: i64,
        input: SendAgentMessageToChildInput,
        runtime_snapshot_required: bool,
        require_runtime_snapshot_for_new_run: bool,
    ) -> Result<AgentMessageAccepted, AppError> {
        self.command(
            user_id,
            AGENT_RUN_SEND_CHILD_OPERATION,
            AgentRunRuntimeCommand {
                input,
                runtime_snapshot_required,
                require_runtime_snapshot_for_new_run,
            },
        )
        .await
    }

    async fn control_child_agent(
        &self,
        user_id: i64,
        input: ControlChildAgentInput,
    ) -> Result<AgentRunView, AppError> {
        self.command(user_id, AGENT_RUN_CONTROL_CHILD_OPERATION, input)
            .await
    }
}
