use super::{AgentApplication, AgentMessageSnapshot, AgentWorkerRun};
use a4n_error::AppError;
use serde_json::Value;

/// Runtime-facing boundary for durable assistant/tool observations of a Run.
#[async_trait::async_trait]
pub trait AgentWorkerMessageApplicationPort: Send + Sync {
    async fn append_tool_message(
        &self,
        claimed: &AgentWorkerRun,
        execution_id: i64,
        tool_name: &str,
        tool_call_id: Option<&str>,
        result: Value,
    ) -> Result<AgentMessageSnapshot, AppError>;

    async fn append_assistant_tool_message(
        &self,
        claimed: &AgentWorkerRun,
        content: String,
        tool_calls: Value,
    ) -> Result<AgentMessageSnapshot, AppError>;
}

#[async_trait::async_trait]
impl AgentWorkerMessageApplicationPort for AgentApplication {
    async fn append_tool_message(
        &self,
        claimed: &AgentWorkerRun,
        execution_id: i64,
        tool_name: &str,
        tool_call_id: Option<&str>,
        result: Value,
    ) -> Result<AgentMessageSnapshot, AppError> {
        AgentApplication::append_tool_message(
            self,
            claimed,
            execution_id,
            tool_name,
            tool_call_id,
            result,
        )
        .await
    }

    async fn append_assistant_tool_message(
        &self,
        claimed: &AgentWorkerRun,
        content: String,
        tool_calls: Value,
    ) -> Result<AgentMessageSnapshot, AppError> {
        AgentApplication::append_assistant_tool_message(self, claimed, content, tool_calls).await
    }
}
