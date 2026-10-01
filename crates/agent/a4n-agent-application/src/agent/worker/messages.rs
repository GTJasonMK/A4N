//! Worker assistant/tool message writes.

use super::super::{AgentApplication, AgentMessageSnapshot, AgentWorkerRun};
use super::*;
use a4n_agent_runtime_contract::AgentWorkerMessageApplicationPort;
use a4n_error::AppError;
use serde_json::Value;

impl AgentApplication {
    pub(crate) async fn append_tool_message(
        &self,
        claimed: &AgentWorkerRun,
        execution_id: i64,
        tool_name: &str,
        tool_call_id: Option<&str>,
        result: Value,
    ) -> Result<AgentMessageSnapshot, AppError> {
        let request = self.worker_message_request_for_claimed(
            claimed,
            AgentWorkerMessageOperation::AppendTool {
                execution_id,
                tool_name: tool_name.to_string(),
                tool_call_id: tool_call_id.map(str::to_string),
                result,
            },
        )?;
        self.apply_message_request(&request)
            .await?
            .ok_or_else(|| AppError::InvalidState {
                current: "Agent tool message 写入没有返回消息".into(),
                operation: "写入 Agent tool message".into(),
            })
    }

    pub(crate) async fn append_assistant_tool_message(
        &self,
        claimed: &AgentWorkerRun,
        content: impl Into<String>,
        tool_calls: Value,
        client_message_id: String,
    ) -> Result<AgentMessageSnapshot, AppError> {
        let request = self.worker_message_request_for_claimed(
            claimed,
            AgentWorkerMessageOperation::AppendAssistantTool {
                content: content.into(),
                tool_calls,
                client_message_id,
            },
        )?;
        self.apply_message_request(&request)
            .await?
            .ok_or_else(|| AppError::InvalidState {
                current: "Agent assistant tool message 写入没有返回消息".into(),
                operation: "写入 Agent assistant tool message".into(),
            })
    }

    pub(crate) async fn upsert_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        content: String,
        reasoning_content: Option<String>,
    ) -> Result<AgentMessageSnapshot, AppError> {
        let request = self.worker_message_request_for_claimed(
            claimed,
            AgentWorkerMessageOperation::UpsertAssistantStreaming {
                client_message_id,
                content,
                reasoning_content,
            },
        )?;
        self.apply_message_request(&request)
            .await?
            .ok_or_else(|| AppError::InvalidState {
                current: "Agent streaming message upsert 没有返回消息".into(),
                operation: "更新 Agent streaming message".into(),
            })
    }

    pub(crate) async fn interrupt_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        reason: Option<String>,
    ) -> Result<(), AppError> {
        let request = self.worker_message_request_for_claimed(
            claimed,
            AgentWorkerMessageOperation::InterruptAssistantStreaming {
                client_message_id,
                reason,
            },
        )?;
        self.apply_message_request(&request).await.map(|_| ())
    }

    pub(crate) async fn complete_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        content: String,
        tool_calls: Value,
    ) -> Result<Option<AgentMessageSnapshot>, AppError> {
        let request = self.worker_message_request_for_claimed(
            claimed,
            AgentWorkerMessageOperation::CompleteAssistantStreaming {
                client_message_id,
                content,
                tool_calls,
            },
        )?;
        self.apply_message_request(&request).await
    }
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
        client_message_id: String,
    ) -> Result<AgentMessageSnapshot, AppError> {
        AgentApplication::append_assistant_tool_message(
            self,
            claimed,
            content,
            tool_calls,
            client_message_id,
        )
        .await
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
}
