//! worker message 与 request application。

use super::*;

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
    ) -> Result<AgentMessageSnapshot, AppError> {
        let request = self.worker_message_request_for_claimed(
            claimed,
            AgentWorkerMessageOperation::AppendAssistantTool {
                content: content.into(),
                tool_calls,
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

    pub(super) fn worker_message_request_for_claimed(
        &self,
        claimed: &AgentWorkerRun,
        operation: AgentWorkerMessageOperation,
    ) -> Result<AgentWorkerMessageRequest, AppError> {
        let worker_id = claimed
            .run
            .worker_id
            .clone()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("Agent run {} 缺少 worker_id", claimed.run.id),
                operation: "构造 Agent message request".into(),
            })?;
        let epoch = u64::try_from(claimed.run.attempt).map_err(|_| AppError::InvalidState {
            current: format!("Agent run {} 的 attempt 无效", claimed.run.id),
            operation: "构造 Agent message request".into(),
        })?;
        let expires_at = claimed
            .run
            .lease_expires_at
            .ok_or_else(|| AppError::InvalidState {
                current: format!("Agent run {} 缺少 lease_expires_at", claimed.run.id),
                operation: "构造 Agent message request".into(),
            })?;
        let request = AgentWorkerMessageRequest {
            run_id: claimed.run.id,
            lease: WorkerLease {
                worker_id,
                epoch,
                fence_token: claimed.lease_token.clone(),
                expires_at: expires_at.to_string(),
            },
            operation,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent message request 无效: {error}"),
            operation: "构造 Agent message request".into(),
        })?;
        Ok(request)
    }

    pub(crate) async fn apply_message_request(
        &self,
        request: &AgentWorkerMessageRequest,
    ) -> Result<Option<AgentMessageSnapshot>, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        let run = agent::run_by_id(self.connection(), request.run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent run 不存在: {}", request.run_id)))?;
        let thread_id = run.thread_id;
        let turn_id = Some(run.turn_id);
        match &request.operation {
            AgentWorkerMessageOperation::AppendTool {
                execution_id,
                tool_name,
                tool_call_id,
                result,
            } => {
                let content = serde_json::to_string(result)?;
                agent::append_message_for_run_lease(
                    self.connection(),
                    request.run_id,
                    &request.lease.fence_token,
                    AgentMessageInput {
                        thread_id,
                        turn_id,
                        run_id: Some(request.run_id),
                        role: "tool".into(),
                        sender_kind: "tool".into(),
                        sender_thread_id: None,
                        client_message_id: Some(format!("agent-tool:{execution_id}")),
                        content,
                        status: "complete".into(),
                        payload_json: Some(json!({
                            "execution_id": execution_id.to_string(),
                            "tool": tool_name,
                            "tool_call_id": tool_call_id,
                            "result": result,
                        })),
                    },
                )
                .await
                .map(message_snapshot)
                .map(Some)
                .map_err(agent_db_error_to_app)
            }
            AgentWorkerMessageOperation::AppendAssistantTool {
                content,
                tool_calls,
            } => agent::append_message_for_run_lease(
                self.connection(),
                request.run_id,
                &request.lease.fence_token,
                AgentMessageInput {
                    thread_id,
                    turn_id,
                    run_id: Some(request.run_id),
                    role: "assistant".into(),
                    sender_kind: "agent".into(),
                    sender_thread_id: None,
                    client_message_id: Some(format!(
                        "agent-assistant:{}:{}",
                        request.run_id,
                        a4n_domain::source_text_hash(&tool_calls.to_string())
                    )),
                    content: content.clone(),
                    status: "complete".into(),
                    payload_json: Some(json!({ "tool_calls": tool_calls })),
                },
            )
            .await
            .map(message_snapshot)
            .map(Some)
            .map_err(agent_db_error_to_app),
            AgentWorkerMessageOperation::UpsertAssistantStreaming {
                client_message_id,
                content,
                reasoning_content,
            } => agent::upsert_assistant_streaming_message_for_run_lease(
                self.connection(),
                request.run_id,
                &request.lease.fence_token,
                AgentMessageInput {
                    thread_id,
                    turn_id,
                    run_id: Some(request.run_id),
                    role: "assistant".into(),
                    sender_kind: "agent".into(),
                    sender_thread_id: None,
                    client_message_id: Some(client_message_id.clone()),
                    content: content.clone(),
                    status: "streaming".into(),
                    payload_json: Some(json!({
                        "streaming": true,
                        "reasoning_content": reasoning_content,
                    })),
                },
            )
            .await
            .map(message_snapshot)
            .map(Some)
            .map_err(agent_db_error_to_app),
            AgentWorkerMessageOperation::InterruptAssistantStreaming {
                client_message_id,
                reason,
            } => agent::interrupt_assistant_streaming_message_for_run_lease(
                self.connection(),
                request.run_id,
                &request.lease.fence_token,
                client_message_id,
                reason.as_deref(),
            )
            .await
            .map(|_| None)
            .map_err(agent_db_error_to_app),
            AgentWorkerMessageOperation::CompleteAssistantStreaming {
                client_message_id,
                content,
                tool_calls,
            } => agent::complete_assistant_streaming_message_for_run_lease(
                self.connection(),
                request.run_id,
                &request.lease.fence_token,
                client_message_id,
                content.clone(),
                json!({ "tool_calls": tool_calls }),
            )
            .await
            .map(|message| message.map(message_snapshot))
            .map_err(agent_db_error_to_app),
        }
    }
}
