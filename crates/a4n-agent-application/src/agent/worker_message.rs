use super::{AgentApplication, AgentMessageSnapshot, AgentWorkerRun};
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerMessageRequest;
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

    async fn upsert_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        content: String,
        reasoning_content: Option<String>,
    ) -> Result<AgentMessageSnapshot, AppError>;

    async fn interrupt_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        reason: Option<String>,
    ) -> Result<(), AppError>;

    async fn complete_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        content: String,
        tool_calls: Value,
    ) -> Result<Option<AgentMessageSnapshot>, AppError>;

    async fn apply_message_request(
        &self,
        request: &AgentWorkerMessageRequest,
    ) -> Result<Option<AgentMessageSnapshot>, AppError>;
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

    async fn upsert_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        content: String,
        reasoning_content: Option<String>,
    ) -> Result<AgentMessageSnapshot, AppError> {
        AgentApplication::upsert_assistant_streaming_message(
            self,
            claimed,
            client_message_id,
            content,
            reasoning_content,
        )
        .await
    }

    async fn interrupt_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        reason: Option<String>,
    ) -> Result<(), AppError> {
        AgentApplication::interrupt_assistant_streaming_message(
            self,
            claimed,
            client_message_id,
            reason,
        )
        .await
    }

    async fn complete_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        content: String,
        tool_calls: Value,
    ) -> Result<Option<AgentMessageSnapshot>, AppError> {
        AgentApplication::complete_assistant_streaming_message(
            self,
            claimed,
            client_message_id,
            content,
            tool_calls,
        )
        .await
    }

    async fn apply_message_request(
        &self,
        request: &AgentWorkerMessageRequest,
    ) -> Result<Option<AgentMessageSnapshot>, AppError> {
        AgentApplication::apply_message_request(self, request).await
    }
}
