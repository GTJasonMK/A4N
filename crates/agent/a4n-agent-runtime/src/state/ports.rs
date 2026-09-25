use super::*;

pub(crate) fn worker_delegation_result_from_wire(
    snapshot: AgentWorkerDelegationStartSnapshotWire,
) -> a4n_agent_runtime_contract::WorkerDelegationResult {
    a4n_agent_runtime_contract::WorkerDelegationResult {
        delegation: snapshot.delegation.into(),
        child_turn: snapshot.child_turn.into(),
        child_run: snapshot.child_run.into(),
        child_message: snapshot.child_message.into(),
    }
}

#[async_trait::async_trait]
impl AgentWorkerExecutionApplicationPort for AgentWorkerExecutionHostPort<'_> {
    async fn worker_execution_by_key(
        &self,
        run_id: i64,
        key: &str,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        if run_id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: format!(
                    "Agent execution run_id 与 claimed run {} 不一致",
                    self.claimed.run.id
                ),
                operation: "读取 Agent execution".into(),
            });
        }
        self.state.worker_execution_by_key(self.claimed, key).await
    }

    async fn create_prepared_execution(
        &self,
        input: WorkerExecutionInput,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        self.state
            .create_prepared_execution(self.claimed, input)
            .await
    }

    async fn update_worker_execution(
        &self,
        input: WorkerExecutionUpdate,
    ) -> Result<AgentExecutionSnapshot, AppError> {
        self.state
            .update_worker_execution(self.claimed, input)
            .await
    }

    async fn worker_execution_by_key_request(
        &self,
        request: &AgentWorkerExecutionLookupRequest,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        self.state
            .host
            .worker_execution_by_key(request)
            .await
            .map(|snapshot| snapshot.map(Into::into))
    }

    async fn create_prepared_execution_request(
        &self,
        request: &AgentWorkerExecutionCreateRequest,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        self.state
            .host
            .create_prepared_execution(request)
            .await
            .map(|snapshot| snapshot.map(Into::into))
    }

    async fn update_worker_execution_request(
        &self,
        request: &AgentWorkerExecutionUpdateRequest,
    ) -> Result<AgentExecutionSnapshot, AppError> {
        self.state
            .host
            .update_worker_execution(request)
            .await
            .map(Into::into)
    }
}

#[async_trait::async_trait]
impl AgentWorkerBudgetApplicationPort for AgentWorkerBudgetHostPort<'_> {
    async fn worker_budget_snapshot(
        &self,
        run_id: i64,
    ) -> Result<AgentWorkerBudgetSnapshot, AppError> {
        if run_id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: format!(
                    "Agent budget run_id 与 claimed run {} 不一致",
                    self.claimed.run.id
                ),
                operation: "读取 Agent budget".into(),
            });
        }
        self.state.worker_budget_snapshot(self.claimed).await
    }

    async fn worker_budget_snapshot_request(
        &self,
        request: &AgentWorkerBudgetRequest,
    ) -> Result<AgentWorkerBudgetSnapshot, AppError> {
        self.state
            .host
            .worker_budget_snapshot(request)
            .await
            .map(Into::into)
    }
}

#[async_trait::async_trait]
impl AgentWorkerApprovalApplicationPort for AgentWorkerApprovalHostPort<'_> {
    async fn approved_execution(
        &self,
        thread_id: i64,
        execution_id: i64,
    ) -> Result<bool, AppError> {
        if thread_id != self.claimed.run.thread_id {
            return Err(AppError::InvalidState {
                current: "Agent approval thread_id 与 claimed run 不一致".into(),
                operation: "读取 Agent approval".into(),
            });
        }
        self.state
            .approved_agent_execution(self.claimed, execution_id)
            .await
    }

    async fn approved_execution_request(
        &self,
        request: &AgentWorkerApprovalCheckRequest,
    ) -> Result<AgentWorkerApprovalCheckSnapshotWire, AppError> {
        if request.run_id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: "Agent approval run_id 与 claimed run 不一致".into(),
                operation: "读取 Agent approval request".into(),
            });
        }
        self.state.host.approved_execution(request).await
    }

    async fn request_worker_approval(
        &self,
        claimed: &AgentWorkerRun,
        execution_id: i64,
        operation_json: Value,
        resource_preconditions_json: Option<Value>,
        expires_at: Option<chrono::NaiveDateTime>,
    ) -> Result<(), AppError> {
        if claimed.run.id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: "Agent approval claimed run 不一致".into(),
                operation: "请求 Agent approval".into(),
            });
        }
        self.state
            .request_agent_worker_approval(
                self.claimed,
                execution_id,
                operation_json,
                resource_preconditions_json,
                expires_at,
            )
            .await
    }

    async fn request_worker_approval_request(
        &self,
        request: &AgentWorkerApprovalRequest,
    ) -> Result<(), AppError> {
        if request.run_id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: "Agent approval run_id 与 claimed run 不一致".into(),
                operation: "请求 Agent approval request".into(),
            });
        }
        self.state.host.request_approval(request).await
    }
}

#[async_trait::async_trait]
impl AgentWorkerFailureApplicationPort for AgentWorkerFailureHostPort<'_> {
    async fn fail_child_run(
        &self,
        claimed: &AgentWorkerRun,
        error_message: String,
    ) -> Result<(), AppError> {
        if claimed.run.id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: "Agent failure claimed run 不一致".into(),
                operation: "持久化 Agent worker failure".into(),
            });
        }
        self.state
            .fail_worker_run(self.claimed, error_message)
            .await
    }

    async fn fail_worker_run(&self, request: &AgentWorkerFailureRequest) -> Result<(), AppError> {
        if request.run_id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: format!(
                    "Agent failure run_id 与 claimed run {} 不一致",
                    self.claimed.run.id
                ),
                operation: "持久化 Agent worker failure request".into(),
            });
        }
        self.state.host.fail_worker_run(request).await
    }
}

impl AgentWorkerMessageHostPort<'_> {
    fn request(
        &self,
        operation: AgentWorkerMessageOperation,
    ) -> Result<AgentWorkerMessageRequest, AppError> {
        let request = AgentWorkerMessageRequest {
            run_id: self.claimed.run.id,
            lease: self.state.agent_worker_lease_request(self.claimed)?.lease,
            operation,
        };
        request.validate().map_err(|error| AppError::InvalidState {
            current: format!("Agent message request 无效: {error}"),
            operation: "构造 Agent message request".into(),
        })?;
        Ok(request)
    }

    async fn apply(
        &self,
        operation: AgentWorkerMessageOperation,
    ) -> Result<Option<AgentMessageSnapshot>, AppError> {
        let request = self.request(operation)?;
        self.state.apply_agent_worker_message(&request).await
    }

    async fn apply_required(
        &self,
        operation: AgentWorkerMessageOperation,
        operation_name: &str,
    ) -> Result<AgentMessageSnapshot, AppError> {
        self.apply(operation)
            .await?
            .ok_or_else(|| AppError::InvalidState {
                current: "Agent message request 没有返回消息".into(),
                operation: operation_name.into(),
            })
    }
}

#[async_trait::async_trait]
impl AgentWorkerMessageApplicationPort for AgentWorkerMessageHostPort<'_> {
    async fn append_tool_message(
        &self,
        claimed: &AgentWorkerRun,
        execution_id: i64,
        tool_name: &str,
        tool_call_id: Option<&str>,
        result: Value,
    ) -> Result<AgentMessageSnapshot, AppError> {
        if claimed.run.id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: "Agent message claimed run 不一致".into(),
                operation: "写入 Agent tool message".into(),
            });
        }
        self.apply_required(
            AgentWorkerMessageOperation::AppendTool {
                execution_id,
                tool_name: tool_name.to_string(),
                tool_call_id: tool_call_id.map(str::to_string),
                result,
            },
            "写入 Agent tool message",
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
        if claimed.run.id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: "Agent message claimed run 不一致".into(),
                operation: "写入 Agent assistant tool message".into(),
            });
        }
        self.apply_required(
            AgentWorkerMessageOperation::AppendAssistantTool {
                content,
                tool_calls,
                client_message_id,
            },
            "写入 Agent assistant tool message",
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
        if claimed.run.id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: "Agent message claimed run 不一致".into(),
                operation: "更新 Agent streaming message".into(),
            });
        }
        self.apply_required(
            AgentWorkerMessageOperation::UpsertAssistantStreaming {
                client_message_id,
                content,
                reasoning_content,
            },
            "更新 Agent streaming message",
        )
        .await
    }

    async fn interrupt_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        reason: Option<String>,
    ) -> Result<(), AppError> {
        if claimed.run.id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: "Agent message claimed run 不一致".into(),
                operation: "中断 Agent streaming message".into(),
            });
        }
        self.apply(AgentWorkerMessageOperation::InterruptAssistantStreaming {
            client_message_id,
            reason,
        })
        .await
        .map(|_| ())
    }

    async fn complete_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        content: String,
        tool_calls: Value,
    ) -> Result<Option<AgentMessageSnapshot>, AppError> {
        if claimed.run.id != self.claimed.run.id {
            return Err(AppError::InvalidState {
                current: "Agent message claimed run 不一致".into(),
                operation: "完成 Agent streaming message".into(),
            });
        }
        self.apply(AgentWorkerMessageOperation::CompleteAssistantStreaming {
            client_message_id,
            content,
            tool_calls,
        })
        .await
    }

    async fn apply_message_request(
        &self,
        request: &AgentWorkerMessageRequest,
    ) -> Result<Option<AgentMessageSnapshot>, AppError> {
        self.state
            .host
            .apply_message_request(request)
            .await
            .map(|snapshot| snapshot.map(Into::into))
    }
}
