use a4n_agent_runtime::{AgentChatCall, AgentChatStream, AgentRuntimeHost};
use a4n_agent_runtime_contract::AgentLlmCapabilities;
use a4n_agent_runtime_contract::AgentWorkerApplicationPort;
use a4n_agent_runtime_contract::model_request_wire_value;
use a4n_api_contract::agent::{
    AgentExecutionResultPage, AgentSkillContent, AgentSkillSearchResult,
};
use a4n_api_contract::chapter_visual_plan::{
    ChapterVisualPlanV3, VisualPlanProjection, VisualPlanReadView,
};
use a4n_api_contract::runtime_config::RuntimeConfigSnapshot;
use a4n_error::AppError;
use a4n_llm_provider_contract::{ChatChunk, ChatRequest};
use a4n_worker_contract::{
    AGENT_WORKER_AUTH_HEADER, AgentHostCommandPayload, AgentRunPayload,
    AgentWorkerApprovalCheckRequest, AgentWorkerApprovalCheckSnapshotWire,
    AgentWorkerApprovalExpiryRequest, AgentWorkerApprovalRequest, AgentWorkerBudgetRequest,
    AgentWorkerBudgetSnapshotWire, AgentWorkerCancelGenerationRequest,
    AgentWorkerCancelWorkflowRequest, AgentWorkerChapterScopeRequest, AgentWorkerChatRequest,
    AgentWorkerClaimRequest, AgentWorkerCompletionRequest, AgentWorkerCompletionSnapshotWire,
    AgentWorkerContextSummaryRequest, AgentWorkerDelegationReviewRequest,
    AgentWorkerDelegationReviewSnapshotWire, AgentWorkerDelegationScopeRequest,
    AgentWorkerDelegationScopeSnapshotWire, AgentWorkerDelegationStartRequest,
    AgentWorkerDelegationStartSnapshotWire, AgentWorkerDelegationThreadRequest,
    AgentWorkerDelegationThreadSnapshotWire, AgentWorkerEditorProjectionRequest,
    AgentWorkerEditorWaitExpiryRequest, AgentWorkerEditorWaitRequest, AgentWorkerEventRequest,
    AgentWorkerExecutionCreateRequest, AgentWorkerExecutionLookupRequest,
    AgentWorkerExecutionResultRequest, AgentWorkerExecutionSnapshotWire,
    AgentWorkerExecutionUpdateRequest, AgentWorkerExpiredRunRecoveryRequest,
    AgentWorkerExternalTaskWaitScanRequest, AgentWorkerExternalTaskWaitSnapshotWire,
    AgentWorkerExternalTaskWaitWakeRequest, AgentWorkerFailureRequest,
    AgentWorkerGenerationCancelScanRequest, AgentWorkerGenerationCancelSnapshotWire,
    AgentWorkerHistoryRequest, AgentWorkerHistorySnapshotWire, AgentWorkerInboxClaimRequest,
    AgentWorkerInboxConsumeRequest, AgentWorkerInboxMessageRequest, AgentWorkerInboxPersistRequest,
    AgentWorkerInboxPersistSnapshotWire, AgentWorkerInboxSnapshotWire,
    AgentWorkerLlmCapabilitiesRequest, AgentWorkerMemoryRequest, AgentWorkerMessageRequest,
    AgentWorkerMessageSnapshotWire, AgentWorkerPackageExecutionResponse,
    AgentWorkerRecoveryRequest, AgentWorkerResultRefsRequest, AgentWorkerResultRefsSnapshotWire,
    AgentWorkerRunContextWire, AgentWorkerRunLeaseRequest, AgentWorkerRunStatusSnapshotWire,
    AgentWorkerRunWaitRequest, AgentWorkerSkillLoadRequest, AgentWorkerSkillSearchRequest,
    AgentWorkerToolDescribeRequest, AgentWorkerToolSearchRequest, AgentWorkerUsageRequest,
    AgentWorkerWaitRequest, AgentWorkerWorkflowCancelScanRequest,
    AgentWorkerWorkflowCancelSnapshotWire, WORKER_SCHEMA_VERSION, WorkerRequest,
};
use async_trait::async_trait;
use futures::stream;
use reqwest::{Client, Method, StatusCode};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct HttpAgentRuntimeHost {
    client: Client,
    base_url: String,
    worker_token: String,
    application: Option<Arc<dyn AgentWorkerApplicationPort>>,
}

impl HttpAgentRuntimeHost {
    pub(crate) fn new(client: Client, base_url: String, worker_token: String) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            worker_token,
            application: None,
        }
    }

    pub(crate) fn with_application(
        mut self,
        application: Arc<dyn AgentWorkerApplicationPort>,
    ) -> Self {
        self.application = Some(application);
        self
    }

    fn application(&self) -> Result<&dyn AgentWorkerApplicationPort, AppError> {
        self.application
            .as_deref()
            .ok_or_else(|| AppError::InvalidState {
                current: "Agent worker 未配置本地 durable application".into(),
                operation: "访问 Agent durable owner".into(),
            })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn request_json<T, B>(&self, method: Method, path: &str, body: &B) -> Result<T, AppError>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let response = self
            .client
            .request(method, self.url(path))
            .header(AGENT_WORKER_AUTH_HEADER, &self.worker_token)
            .json(body)
            .send()
            .await
            .map_err(|error| {
                AppError::ConfigError(format!("Control Agent Host 请求失败: {error}"))
            })?;
        let status = response.status();
        let body = response.text().await.map_err(|error| {
            AppError::ConfigError(format!("读取 Control Agent Host 响应失败: {error}"))
        })?;
        if !status.is_success() {
            return Err(AppError::InvalidState {
                current: format!("Control Agent Host 返回 {status}: {body}"),
                operation: format!("调用 Agent Host {path}"),
            });
        }
        serde_json::from_str(&body).map_err(|error| {
            AppError::Serialization(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Agent Host 响应不是有效 JSON: {error}"),
            )))
        })
    }

    async fn request_stream(
        &self,
        body: &AgentWorkerChatRequest,
    ) -> Result<AgentChatStream, AppError> {
        let response = self
            .client
            .post(self.url("/internal/agent/llm/chat"))
            .header(AGENT_WORKER_AUTH_HEADER, &self.worker_token)
            .json(body)
            .send()
            .await
            .map_err(|error| AppError::LlmError(format!("Control Agent LLM 请求失败: {error}")))?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::LlmError(format!(
                "Control Agent LLM 返回 {status}: {body}"
            )));
        }
        Ok(chat_sse_stream(response))
    }
}

#[async_trait]
impl AgentRuntimeHost for HttpAgentRuntimeHost {
    async fn claim_worker(
        &self,
        request: &AgentWorkerClaimRequest,
    ) -> Result<Option<WorkerRequest<AgentRunPayload>>, AppError> {
        let response = self
            .client
            .post(self.url("/internal/agent/claim"))
            .header(AGENT_WORKER_AUTH_HEADER, &self.worker_token)
            .json(request)
            .send()
            .await
            .map_err(|error| {
                AppError::ConfigError(format!("Control Agent claim 请求失败: {error}"))
            })?;
        if response.status() == StatusCode::NO_CONTENT {
            return Ok(None);
        }
        let status = response.status();
        let body = response.text().await.map_err(|error| {
            AppError::ConfigError(format!("读取 Control Agent claim 响应失败: {error}"))
        })?;
        if !status.is_success() {
            return Err(AppError::InvalidState {
                current: format!("Control Agent claim 返回 {status}: {body}"),
                operation: "领取 Agent worker run".into(),
            });
        }
        serde_json::from_str(&body).map(Some).map_err(|error| {
            AppError::Serialization(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Agent claim 响应不是有效 JSON: {error}"),
            )))
        })
    }

    async fn load_run_context(
        &self,
        request: &WorkerRequest<AgentRunPayload>,
    ) -> Result<AgentWorkerRunContextWire, AppError> {
        let claimed = a4n_agent_runtime_contract::AgentWorkerRun::from_wire(
            request.payload.worker_run.clone(),
        )?;
        self.application()?
            .load_run_context(&claimed)
            .await
            .map(|context| context.to_wire())
    }

    async fn renew_worker_lease(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<(), AppError> {
        self.application()?
            .renew_worker_lease_request(request)
            .await
    }

    async fn mark_turn_active(&self, request: &AgentWorkerRunLeaseRequest) -> Result<(), AppError> {
        self.application()?.mark_turn_active_request(request).await
    }

    async fn update_run_usage(&self, request: &AgentWorkerUsageRequest) -> Result<(), AppError> {
        self.application()?.update_run_usage_request(request).await
    }

    async fn wait_run(&self, request: &AgentWorkerRunWaitRequest) -> Result<(), AppError> {
        self.application()?.wait_run_request(request).await
    }

    async fn update_context_summary(
        &self,
        request: &AgentWorkerContextSummaryRequest,
    ) -> Result<(), AppError> {
        self.application()?
            .update_context_summary_request(request)
            .await
    }

    async fn worker_execution_by_key(
        &self,
        request: &AgentWorkerExecutionLookupRequest,
    ) -> Result<Option<AgentWorkerExecutionSnapshotWire>, AppError> {
        self.application()?
            .worker_execution_by_key_request(request)
            .await
            .map(|snapshot| snapshot.map(Into::into))
    }

    async fn create_prepared_execution(
        &self,
        request: &AgentWorkerExecutionCreateRequest,
    ) -> Result<Option<AgentWorkerExecutionSnapshotWire>, AppError> {
        self.application()?
            .create_prepared_execution_request(request)
            .await
            .map(|snapshot| snapshot.map(Into::into))
    }

    async fn update_worker_execution(
        &self,
        request: &AgentWorkerExecutionUpdateRequest,
    ) -> Result<AgentWorkerExecutionSnapshotWire, AppError> {
        self.application()?
            .update_worker_execution_request(request)
            .await
            .map(Into::into)
    }

    async fn read_execution_result(
        &self,
        request: &AgentWorkerExecutionResultRequest,
    ) -> Result<AgentExecutionResultPage, AppError> {
        self.application()?
            .read_execution_result_request(request)
            .await
    }

    async fn read_memory(
        &self,
        request: &AgentWorkerMemoryRequest,
    ) -> Result<a4n_api_contract::agent::AgentMemorySearchResult, AppError> {
        self.application()?.read_memory_request(request).await
    }

    async fn search_agent_tools(
        &self,
        request: &AgentWorkerToolSearchRequest,
    ) -> Result<a4n_api_contract::agent::AgentToolSearchResult, AppError> {
        self.application()?
            .search_tools_for_worker(request.input.clone())
    }

    async fn describe_agent_tool(
        &self,
        request: &AgentWorkerToolDescribeRequest,
    ) -> Result<a4n_api_contract::agent::AgentToolDescriptor, AppError> {
        self.application()?
            .describe_tool_for_worker(request.input.clone())
    }

    async fn search_agent_skills(
        &self,
        request: &AgentWorkerSkillSearchRequest,
    ) -> Result<AgentSkillSearchResult, AppError> {
        self.application()?.search_skills_for_worker(request).await
    }

    async fn load_agent_skill(
        &self,
        request: &AgentWorkerSkillLoadRequest,
    ) -> Result<AgentSkillContent, AppError> {
        self.application()?.load_skill_for_worker(request).await
    }

    async fn verify_agent_chapters(
        &self,
        request: &AgentWorkerChapterScopeRequest,
    ) -> Result<(), AppError> {
        self.application()?
            .verify_chapters_in_project_for_worker(
                request.user_id,
                request.project_id,
                &request.chapter_ids,
            )
            .await
    }

    async fn read_worker_history(
        &self,
        request: &AgentWorkerHistoryRequest,
    ) -> Result<AgentWorkerHistorySnapshotWire, AppError> {
        self.application()?.worker_history_request(request).await
    }

    async fn mark_recovery_required(
        &self,
        request: &AgentWorkerRecoveryRequest,
    ) -> Result<(), AppError> {
        self.application()?
            .mark_recovery_required_request(request)
            .await
    }

    async fn wait_for_editor(
        &self,
        request: &AgentWorkerEditorWaitRequest,
    ) -> Result<(), AppError> {
        self.application()?.wait_for_editor_request(request).await
    }

    async fn expire_editor_waits(
        &self,
        request: &AgentWorkerEditorWaitExpiryRequest,
    ) -> Result<usize, AppError> {
        self.application()?
            .expire_editor_requests(request.limit)
            .await
    }

    async fn expire_pending_approvals(
        &self,
        request: &AgentWorkerApprovalExpiryRequest,
    ) -> Result<usize, AppError> {
        self.application()?
            .expire_pending_approvals(request.limit)
            .await
    }

    async fn recover_expired_runs(
        &self,
        request: &AgentWorkerExpiredRunRecoveryRequest,
    ) -> Result<usize, AppError> {
        self.application()?
            .recover_expired_runs(request.limit, request.error_message.as_deref())
            .await
    }

    async fn generation_cancel_candidates(
        &self,
        request: &AgentWorkerGenerationCancelScanRequest,
    ) -> Result<Vec<AgentWorkerGenerationCancelSnapshotWire>, AppError> {
        self.application()?
            .generation_cancel_candidates(&request.after_task_id)
            .await
            .map(|items| {
                items
                    .into_iter()
                    .map(|item| AgentWorkerGenerationCancelSnapshotWire {
                        task_id: item.task_id,
                        attempt: item.attempt,
                        requested_by_user_id: item.requested_by_user_id,
                        status: item.status,
                        owner_stopped: item.owner_stopped,
                    })
                    .collect()
            })
    }

    async fn workflow_cancel_candidates(
        &self,
        request: &AgentWorkerWorkflowCancelScanRequest,
    ) -> Result<Vec<AgentWorkerWorkflowCancelSnapshotWire>, AppError> {
        self.application()?
            .workflow_cancel_candidates(request.after_workflow_run_id)
            .await
            .map(|items| {
                items
                    .into_iter()
                    .map(|item| AgentWorkerWorkflowCancelSnapshotWire {
                        workflow_run_id: item.workflow_run_id,
                        owner_stopped: item.owner_stopped,
                    })
                    .collect()
            })
    }

    async fn external_task_waits(
        &self,
        request: &AgentWorkerExternalTaskWaitScanRequest,
    ) -> Result<Vec<AgentWorkerExternalTaskWaitSnapshotWire>, AppError> {
        self.application()?
            .external_task_waits(request.after_run_id)
            .await
            .map(|items| {
                items
                    .into_iter()
                    .map(|item| AgentWorkerExternalTaskWaitSnapshotWire {
                        run_id: item.run_id,
                        owner_user_id: item.owner_user_id,
                        scope: item.scope,
                        execution_id: item.execution_id,
                        tool_name: item.tool_name,
                        arguments_json: item.arguments_json,
                        duration_exhausted: item.duration_exhausted,
                    })
                    .collect()
            })
    }

    async fn wake_external_task_wait(
        &self,
        request: &AgentWorkerExternalTaskWaitWakeRequest,
    ) -> Result<(), AppError> {
        self.application()?
            .wake_external_task_wait(request.run_id, request.execution_id)
            .await
    }

    async fn claim_inbox(
        &self,
        request: &AgentWorkerInboxClaimRequest,
    ) -> Result<Option<AgentWorkerInboxSnapshotWire>, AppError> {
        self.application()?.claim_inbox_request(request).await
    }

    async fn consume_inbox(
        &self,
        request: &AgentWorkerInboxConsumeRequest,
    ) -> Result<AgentWorkerInboxSnapshotWire, AppError> {
        self.application()?.consume_inbox_request(request).await
    }

    async fn persist_inbox(
        &self,
        request: &AgentWorkerInboxPersistRequest,
    ) -> Result<AgentWorkerInboxPersistSnapshotWire, AppError> {
        self.application()?.persist_inbox_request(request).await
    }

    async fn load_inbox_message(
        &self,
        request: &AgentWorkerInboxMessageRequest,
    ) -> Result<AgentWorkerMessageSnapshotWire, AppError> {
        self.application()?.load_message_request(request).await
    }

    async fn apply_message_request(
        &self,
        request: &AgentWorkerMessageRequest,
    ) -> Result<Option<AgentWorkerMessageSnapshotWire>, AppError> {
        self.application()?
            .apply_message_request(request)
            .await
            .map(|snapshot| snapshot.map(Into::into))
    }

    async fn worker_budget_snapshot(
        &self,
        request: &AgentWorkerBudgetRequest,
    ) -> Result<AgentWorkerBudgetSnapshotWire, AppError> {
        self.application()?
            .worker_budget_snapshot_request(request)
            .await
            .map(Into::into)
    }

    async fn fail_worker_run(&self, request: &AgentWorkerFailureRequest) -> Result<(), AppError> {
        self.application()?.fail_worker_run(request).await
    }

    async fn result_refs_for_run(
        &self,
        request: &AgentWorkerResultRefsRequest,
    ) -> Result<AgentWorkerResultRefsSnapshotWire, AppError> {
        self.application()?
            .result_refs_for_run_request(request)
            .await
    }

    async fn complete_worker_run(
        &self,
        request: &AgentWorkerCompletionRequest,
    ) -> Result<AgentWorkerCompletionSnapshotWire, AppError> {
        self.application()?
            .complete_worker_run_request(request)
            .await
    }

    async fn approved_execution(
        &self,
        request: &AgentWorkerApprovalCheckRequest,
    ) -> Result<AgentWorkerApprovalCheckSnapshotWire, AppError> {
        self.application()?
            .approved_execution_request(request)
            .await
    }

    async fn request_approval(&self, request: &AgentWorkerApprovalRequest) -> Result<(), AppError> {
        self.application()?
            .request_worker_approval_request(request)
            .await
    }

    async fn review_delegation(
        &self,
        request: &AgentWorkerDelegationReviewRequest,
    ) -> Result<AgentWorkerDelegationReviewSnapshotWire, AppError> {
        self.application()?.review_delegation_request(request).await
    }

    async fn inherit_agent_delegation_scope(
        &self,
        request: &AgentWorkerDelegationScopeRequest,
    ) -> Result<AgentWorkerDelegationScopeSnapshotWire, AppError> {
        let mut scope = request.child_scope.clone();
        self.application()?
            .inherit_subagent_allowlists(&request.parent_scope, &mut scope)?;
        Ok(AgentWorkerDelegationScopeSnapshotWire { scope })
    }

    async fn ensure_canonical_subagent_thread(
        &self,
        request: &AgentWorkerDelegationThreadRequest,
    ) -> Result<AgentWorkerDelegationThreadSnapshotWire, AppError> {
        self.application()?
            .ensure_canonical_subagent_thread_request(request)
            .await
    }

    async fn append_worker_update(
        &self,
        request: &AgentWorkerEventRequest,
    ) -> Result<(), AppError> {
        self.application()?
            .append_worker_update_request(request)
            .await
    }

    async fn create_child_run(
        &self,
        request: &AgentWorkerDelegationStartRequest,
    ) -> Result<AgentWorkerDelegationStartSnapshotWire, AppError> {
        self.application()?.create_child_run_request(request).await
    }

    async fn run_is_terminal(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<AgentWorkerRunStatusSnapshotWire, AppError> {
        self.application()?.run_is_terminal_request(request).await
    }

    async fn execute_command(
        &self,
        _user_id: i64,
        _command: String,
        _arguments: Value,
        request: WorkerRequest<AgentHostCommandPayload>,
    ) -> Result<Value, AppError> {
        self.request_json(Method::POST, "/internal/agent/command", &request)
            .await
    }

    async fn package_execution_enabled(&self) -> Result<bool, AppError> {
        self.request_json(Method::GET, "/internal/agent/package", &())
            .await
            .map(|response: AgentWorkerPackageExecutionResponse| response.enabled)
    }

    async fn cancel_generation_task(
        &self,
        user_id: i64,
        task_id: String,
        attempt: u32,
    ) -> Result<(), AppError> {
        let _: Value = self
            .request_json(
                Method::POST,
                "/internal/agent/generation/cancel",
                &AgentWorkerCancelGenerationRequest {
                    user_id,
                    task_id,
                    attempt,
                },
            )
            .await?;
        Ok(())
    }

    async fn cancel_workflow(&self, workflow_run_id: i64, reason: &str) -> Result<(), AppError> {
        let _: Value = self
            .request_json(
                Method::POST,
                "/internal/agent/workflow/cancel",
                &AgentWorkerCancelWorkflowRequest {
                    workflow_run_id,
                    reason: reason.to_string(),
                },
            )
            .await?;
        Ok(())
    }

    async fn read_external_task_wait(
        &self,
        wait: &AgentWorkerExternalTaskWaitSnapshotWire,
        snapshot: Option<&RuntimeConfigSnapshot>,
    ) -> Result<Value, AppError> {
        let request = AgentWorkerWaitRequest {
            run_id: wait.run_id,
            owner_user_id: wait.owner_user_id,
            scope: wait.scope.clone(),
            execution_id: wait.execution_id,
            tool_name: wait.tool_name.clone(),
            arguments_json: wait.arguments_json.clone(),
            duration_exhausted: wait.duration_exhausted,
            runtime_config_snapshot: snapshot.cloned(),
        };
        self.request_json(Method::POST, "/internal/agent/wait", &request)
            .await
    }

    async fn llm_capabilities(
        &self,
        user_id: i64,
        snapshot: Option<&RuntimeConfigSnapshot>,
        config_id: Option<i64>,
    ) -> Result<AgentLlmCapabilities, AppError> {
        self.request_json(
            Method::POST,
            "/internal/agent/llm/capabilities",
            &AgentWorkerLlmCapabilitiesRequest {
                schema_version: WORKER_SCHEMA_VERSION,
                user_id,
                snapshot: snapshot.cloned(),
                config_id,
            },
        )
        .await
    }

    async fn chat_stream(
        &self,
        user_id: i64,
        snapshot: Option<&RuntimeConfigSnapshot>,
        config_id: Option<i64>,
        request: ChatRequest,
    ) -> Result<AgentChatCall, AppError> {
        let prepared_request = request.clone();
        let request = model_request_wire_value(&request)?;
        let stream = self
            .request_stream(&AgentWorkerChatRequest {
                schema_version: WORKER_SCHEMA_VERSION,
                user_id,
                snapshot: snapshot.cloned(),
                config_id,
                request,
            })
            .await?;
        Ok(AgentChatCall {
            request: prepared_request,
            stream,
        })
    }

    async fn prepare_chat_request(
        &self,
        user_id: i64,
        snapshot: Option<&RuntimeConfigSnapshot>,
        config_id: Option<i64>,
        request: ChatRequest,
    ) -> Result<ChatRequest, AppError> {
        let value = self
            .request_json(
                Method::POST,
                "/internal/agent/llm/prepare",
                &AgentWorkerChatRequest {
                    schema_version: WORKER_SCHEMA_VERSION,
                    user_id,
                    snapshot: snapshot.cloned(),
                    config_id,
                    request: model_request_wire_value(&request)?,
                },
            )
            .await?;
        a4n_agent_runtime_contract::restore_model_request_wire(value)
    }

    async fn project_editor_draft(
        &self,
        plan: &ChapterVisualPlanV3,
        view: &VisualPlanReadView,
    ) -> Result<VisualPlanProjection, AppError> {
        self.request_json(
            Method::POST,
            "/internal/agent/editor/project",
            &AgentWorkerEditorProjectionRequest {
                plan: plan.clone(),
                view: view.clone(),
            },
        )
        .await
    }
}

struct ChatSseState {
    response: reqwest::Response,
    buffer: String,
    finished: bool,
}

fn chat_sse_stream(response: reqwest::Response) -> AgentChatStream {
    Box::pin(stream::unfold(
        ChatSseState {
            response,
            buffer: String::new(),
            finished: false,
        },
        |mut state| async move {
            loop {
                if let Some(item) = state.next_event(false) {
                    return Some((item, state));
                }
                if state.finished {
                    return state.next_event(true).map(|item| (item, state));
                }
                match state.response.chunk().await {
                    Ok(Some(bytes)) => {
                        state.buffer.push_str(&String::from_utf8_lossy(&bytes));
                    }
                    Ok(None) => state.finished = true,
                    Err(error) => {
                        state.finished = true;
                        return Some((
                            Err(AppError::LlmError(format!(
                                "读取 Control Agent LLM 流失败: {error}"
                            ))),
                            state,
                        ));
                    }
                }
            }
        },
    ))
}

impl ChatSseState {
    fn next_event(&mut self, flush: bool) -> Option<Result<ChatChunk, AppError>> {
        let end = self
            .buffer
            .find("\n\n")
            .or_else(|| flush.then_some(self.buffer.len()))?;
        if end == 0 && !flush {
            return None;
        }
        let raw = self.buffer[..end].to_string();
        let consumed = end.saturating_add(2).min(self.buffer.len());
        self.buffer.drain(..consumed);
        let mut event_name = "message";
        let mut data = Vec::new();
        for line in raw.lines() {
            let line = line.trim_end_matches('\r');
            if let Some(value) = line.strip_prefix("event:") {
                event_name = value.trim();
            } else if let Some(value) = line.strip_prefix("data:") {
                data.push(value.trim_start().to_string());
            }
        }
        if data.is_empty() {
            return None;
        }
        let data = data.join("\n");
        if event_name == "error" {
            return Some(Err(AppError::LlmError(
                serde_json::from_str::<Value>(&data)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("message")
                            .or_else(|| value.get("error"))
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
                    .unwrap_or(data),
            )));
        }
        Some(
            serde_json::from_str(&data).map_err(|error| {
                AppError::LlmError(format!("Control Agent LLM 流数据无效: {error}"))
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_llm_provider_contract::LlmTraceContext;
    use serde_json::json;

    #[test]
    fn agent_chat_wire_request_preserves_internal_context() {
        let mut request = ChatRequest::system_user("system", "user", Some(0.2), Some(128));
        request.trace_context = Some(LlmTraceContext {
            project_id: Some(7),
            root_project_id: Some(3),
            workflow: "agent".into(),
            step_key: "model".into(),
            generation_run_id: None,
            workflow_run_id: None,
            call_key: Some("call-1".into()),
            input_fingerprint_sha256: None,
            context_plan_json: None,
            context_pack_refs_json: None,
        });
        request.context_manifest_json = Some(json!({"packs": []}));

        let wire = model_request_wire_value(&request).expect("wire request serializes");
        assert_eq!(wire["trace_context"]["project_id"], 7);
        assert_eq!(wire["context_manifest_json"], json!({"packs": []}));
        assert!(
            request
                .snapshot_value()
                .expect("request snapshot serializes")
                .get("context_manifest_json")
                .is_some()
        );
    }
}
