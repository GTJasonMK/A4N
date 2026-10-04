//! worker run completion and result references.

use super::super::{AgentApplication, AgentWorkerRun};
use super::*;
use a4n_agent_runtime_contract::AgentWorkerCompletionApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentWorkerCompletionRequest, AgentWorkerCompletionSnapshotWire, AgentWorkerResultRefsRequest,
    AgentWorkerResultRefsSnapshotWire, AgentWorkerRunLeaseRequest,
};
use serde_json::Value;

impl AgentApplication {
    pub(crate) async fn result_refs_for_run(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<Value, AppError> {
        self.result_refs_for_run_id(claimed.run.id).await
    }

    pub(super) async fn result_refs_for_run_id(&self, run_id: i64) -> Result<Value, AppError> {
        let executions = agent::executions_by_runs(self.executor(), vec![run_id])
            .await
            .map_err(agent_db_error_to_app)?;
        let mut refs = Vec::<AgentResourceRef>::new();
        for execution in executions
            .into_iter()
            .filter(|execution| execution.status == "succeeded")
        {
            if let Some(value) = execution.result_refs_json {
                let items: Vec<AgentResourceRef> =
                    from_json(value, "agent_execution.result_refs_json")?;
                for item in items {
                    if item.kind == "optimization_source" {
                        continue;
                    }
                    refs.retain(|previous| previous.kind != item.kind || previous.id != item.id);
                    refs.push(item);
                }
            }
        }
        serde_json::to_value(refs).map_err(AppError::from)
    }

    pub(crate) async fn result_refs_for_run_request(
        &self,
        request: &AgentWorkerResultRefsRequest,
    ) -> Result<AgentWorkerResultRefsSnapshotWire, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        self.result_refs_for_run_id(request.run_id)
            .await
            .map(|result_refs_json| AgentWorkerResultRefsSnapshotWire { result_refs_json })
    }

    pub(super) async fn complete_claimed_run_fields(
        &self,
        fields: RunLeaseFields<'_>,
        source_execution_id: i64,
        content: String,
        result_refs_json: Option<Value>,
        observed_input_message_id: i64,
    ) -> Result<AgentWorkerCompletionSnapshotWire, AppError> {
        let completed = agent::finish_run_with_message(
            self.executor(),
            fields.run_id,
            fields.lease_token,
            AgentMessageInput {
                thread_id: fields.thread_id,
                turn_id: Some(fields.turn_id),
                run_id: Some(fields.run_id),
                role: "assistant".into(),
                sender_kind: "agent".into(),
                sender_thread_id: None,
                client_message_id: Some(format!("agent-final:{}", fields.run_id)),
                content,
                status: "complete".into(),
                payload_json: Some(json!({"completion_execution_id": source_execution_id})),
            },
            result_refs_json,
            observed_input_message_id,
        )
        .await;
        completion_receipt(completed)
    }

    pub(super) async fn complete_child_run_fields(
        &self,
        fields: RunLeaseFields<'_>,
        source_execution_id: i64,
        content: String,
        result_summary: String,
        result_refs_json: Option<Value>,
        observed_input_message_id: i64,
    ) -> Result<AgentWorkerCompletionSnapshotWire, AppError> {
        completion_receipt(
            agent::complete_child_delegation(
                self.executor(),
                a4n_db::agent::AgentChildCompletionInput {
                    child_run_id: fields.run_id,
                    child_lease_token: fields.lease_token.to_string(),
                    final_message: AgentMessageInput {
                        thread_id: fields.thread_id,
                        turn_id: Some(fields.turn_id),
                        run_id: Some(fields.run_id),
                        role: "assistant".into(),
                        sender_kind: "agent".into(),
                        sender_thread_id: None,
                        client_message_id: Some(format!("agent-final:{}", fields.run_id)),
                        content,
                        status: "complete".into(),
                        payload_json: Some(json!({"completion_execution_id": source_execution_id})),
                    },
                    result_refs_json,
                    result_summary,
                    observed_input_message_id,
                },
            )
            .await,
        )
    }

    pub(crate) async fn complete_worker_run_request(
        &self,
        request: &AgentWorkerCompletionRequest,
    ) -> Result<AgentWorkerCompletionSnapshotWire, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let run = agent::run_by_id(self.executor(), request.run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent run 不存在: {}", request.run_id)))?;
        let source = agent::execution_by_id(self.executor(), request.source_execution_id)
            .await
            .map_err(agent_db_error_to_app)?
            .filter(|row| {
                row.run_id == run.id
                    && row.status == "succeeded"
                    && row.execution_key.starts_with("model-call:")
            })
            .ok_or_else(|| AppError::InvalidArgument("完成请求缺少本 Run 的成功模型执行".into()))?;
        if source
            .resource_preconditions_json
            .as_ref()
            .and_then(|value| value.get("input_message_id"))
            .and_then(Value::as_i64)
            != Some(request.observed_input_message_id)
        {
            return Err(AppError::InvalidArgument(
                "完成请求的消息边界与模型执行不一致".into(),
            ));
        }
        let response = serde_json::from_value(
            source
                .result_json
                .clone()
                .ok_or_else(|| AppError::InvalidArgument("模型执行缺少响应".into()))?,
        )?;
        let protocol = source
            .arguments_json
            .as_ref()
            .map(a4n_agent_runtime_contract::agent_protocol_from_request_snapshot)
            .ok_or_else(|| AppError::InvalidArgument("模型执行缺少请求".into()))?;
        if !matches!(a4n_agent_runtime_contract::decode_agent_response(&response, protocol, source.id)?, a4n_agent_runtime_contract::AgentModelDecision::Final { reply } if reply == request.content)
        {
            return Err(AppError::InvalidArgument(
                "完成内容与已保存的模型决定不一致".into(),
            ));
        }
        let feedback_client_id = format!("agent-notice:{}", source.id);
        if let Some(feedback) =
            agent::message_by_run_and_client_id(self.executor(), run.id, &feedback_client_id)
                .await
                .map_err(agent_db_error_to_app)?
        {
            let receipt = feedback
                .payload_json
                .and_then(|payload| payload.get("completion_receipt").cloned())
                .ok_or_else(|| AppError::InvalidState {
                    current: "Agent 完成反馈缺少回执".into(),
                    operation: "恢复完成检查".into(),
                })?;
            return serde_json::from_value(receipt).map_err(AppError::from);
        }
        if run.status == "completed" {
            let finished = agent::turn_by_id(self.executor(), run.turn_id)
                .await
                .map_err(agent_db_error_to_app)?
                .and_then(|turn| turn.final_message_id);
            if let Some(finished) = finished {
                let message = agent::message_by_id(self.executor(), finished)
                    .await
                    .map_err(agent_db_error_to_app)?;
                if message.is_some_and(|message| {
                    message.content == request.content
                        && message
                            .payload_json
                            .as_ref()
                            .and_then(|v| v.get("completion_execution_id"))
                            .and_then(Value::as_i64)
                            == Some(source.id)
                }) {
                    return Ok(AgentWorkerCompletionSnapshotWire {
                        completed: true,
                        blockers: Vec::new(),
                    });
                }
            }
            return Err(AppError::InvalidArgument(
                "Run 已由另一个模型决定完成".into(),
            ));
        }
        if run.status == "queued"
            && agent::has_user_message_after(
                self.executor(),
                run.id,
                request.observed_input_message_id,
            )
            .await
            .map_err(agent_db_error_to_app)?
        {
            return Ok(AgentWorkerCompletionSnapshotWire {
                completed: false,
                blockers: Vec::new(),
            });
        }
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        let receipt = if run.parent_run_id.is_some() {
            self.complete_child_run_fields(
                RunLeaseFields {
                    run_id: run.id,
                    thread_id: run.thread_id,
                    turn_id: run.turn_id,
                    lease_token: &request.lease.fence_token,
                },
                source.id,
                request.content.clone(),
                request.result_summary.clone(),
                request.result_refs_json.clone(),
                request.observed_input_message_id,
            )
            .await?
        } else {
            self.complete_claimed_run_fields(
                RunLeaseFields {
                    run_id: run.id,
                    thread_id: run.thread_id,
                    turn_id: run.turn_id,
                    lease_token: &request.lease.fence_token,
                },
                source.id,
                request.content.clone(),
                request.result_refs_json.clone(),
                request.observed_input_message_id,
            )
            .await?
        };
        if !receipt.blockers.is_empty() {
            agent::append_message_for_run_lease(
                self.executor(),
                run.id,
                &request.lease.fence_token,
                AgentMessageInput {
                    thread_id: run.thread_id,
                    turn_id: Some(run.turn_id),
                    run_id: Some(run.id),
                    role: "system".into(),
                    sender_kind: "agent".into(),
                    sender_thread_id: None,
                    client_message_id: Some(format!("agent-notice:{}", source.id)),
                    content: format!(
                        "任务还有 {} 项未完成，继续处理后再交付。",
                        receipt.blockers.len()
                    ),
                    status: "complete".into(),
                    payload_json: Some(
                        json!({"notice_execution_id": source.id, "completion_receipt": receipt}),
                    ),
                },
            )
            .await
            .map_err(agent_db_error_to_app)?;
        }
        Ok(receipt)
    }
}

#[async_trait::async_trait]
impl AgentWorkerCompletionApplicationPort for AgentApplication {
    async fn result_refs_for_run(&self, claimed: &AgentWorkerRun) -> Result<Value, AppError> {
        AgentApplication::result_refs_for_run(self, claimed).await
    }

    async fn result_refs_for_run_request(
        &self,
        request: &AgentWorkerResultRefsRequest,
    ) -> Result<AgentWorkerResultRefsSnapshotWire, AppError> {
        AgentApplication::result_refs_for_run_request(self, request).await
    }

    async fn complete_worker_run_request(
        &self,
        request: &AgentWorkerCompletionRequest,
    ) -> Result<AgentWorkerCompletionSnapshotWire, AppError> {
        AgentApplication::complete_worker_run_request(self, request).await
    }
}

pub(super) fn completion_receipt<T>(
    result: Result<Option<T>, AgentDbError>,
) -> Result<AgentWorkerCompletionSnapshotWire, AppError> {
    match result {
        Ok(result) => Ok(AgentWorkerCompletionSnapshotWire {
            completed: result.is_some(),
            blockers: Vec::new(),
        }),
        Err(AgentDbError::CompletionBlocked(blockers)) => Ok(AgentWorkerCompletionSnapshotWire {
            completed: false,
            blockers,
        }),
        Err(error) => Err(agent_db_error_to_app(error)),
    }
}
