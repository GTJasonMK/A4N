use a4n_agent_application::agent::{AgentExternalTaskWaitSnapshot, AgentRunContext};
use a4n_agent_runtime::{AgentChatStream, AgentRuntimeHost};
use a4n_api_contract::agent::AgentExecutionResultPage;
use a4n_api_contract::chapter_visual_plan::{
    ChapterVisualPlanV2, VisualPlanProjection, VisualPlanReadView,
};
use a4n_api_contract::runtime_config::RuntimeConfigSnapshot;
use a4n_error::AppError;
use a4n_llm::provider::{ChatChunk, ChatRequest, ModelCapabilities};
use a4n_worker_contract::{
    AGENT_WORKER_AUTH_HEADER, AgentHostCommandPayload, AgentRunPayload,
    AgentWorkerApprovalCheckRequest, AgentWorkerApprovalCheckSnapshotWire,
    AgentWorkerApprovalRequest, AgentWorkerBudgetRequest, AgentWorkerBudgetSnapshotWire,
    AgentWorkerCancelGenerationRequest, AgentWorkerCancelWorkflowRequest, AgentWorkerChatRequest,
    AgentWorkerCompletionRequest, AgentWorkerCompletionSnapshotWire,
    AgentWorkerContextSummaryRequest, AgentWorkerDelegationReviewRequest,
    AgentWorkerDelegationReviewSnapshotWire, AgentWorkerDelegationStartRequest,
    AgentWorkerDelegationStartSnapshotWire, AgentWorkerDelegationThreadRequest,
    AgentWorkerDelegationThreadSnapshotWire, AgentWorkerEditorProjectionRequest,
    AgentWorkerEventRequest, AgentWorkerExecutionCreateRequest, AgentWorkerExecutionLookupRequest,
    AgentWorkerExecutionResultRequest, AgentWorkerExecutionSnapshotWire,
    AgentWorkerExecutionUpdateRequest, AgentWorkerFailureRequest,
    AgentWorkerLlmCapabilitiesRequest, AgentWorkerMessageRequest, AgentWorkerMessageSnapshotWire,
    AgentWorkerPackageExecutionResponse, AgentWorkerResultRefsRequest,
    AgentWorkerResultRefsSnapshotWire, AgentWorkerRunContextWire, AgentWorkerRunLeaseRequest,
    AgentWorkerRunStatusSnapshotWire, AgentWorkerRunWaitRequest, AgentWorkerUsageRequest,
    AgentWorkerWaitRequest, WorkerRequest,
};
use async_trait::async_trait;
use futures::stream;
use reqwest::{Client, Method};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

#[derive(Clone)]
pub(crate) struct HttpAgentRuntimeHost {
    client: Client,
    base_url: String,
    worker_token: String,
}

impl HttpAgentRuntimeHost {
    pub(crate) fn new(client: Client, base_url: String, worker_token: String) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            worker_token,
        }
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
    async fn load_run_context(
        &self,
        request: &WorkerRequest<AgentRunPayload>,
    ) -> Result<AgentRunContext, AppError> {
        let context: AgentWorkerRunContextWire = self
            .request_json(Method::POST, "/internal/agent/context", request)
            .await?;
        Ok(AgentRunContext::from_wire(context))
    }

    async fn renew_worker_lease(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<(), AppError> {
        let _: Value = self
            .request_json(Method::POST, "/internal/agent/lease/renew", request)
            .await?;
        Ok(())
    }

    async fn mark_turn_active(&self, request: &AgentWorkerRunLeaseRequest) -> Result<(), AppError> {
        let _: Value = self
            .request_json(Method::POST, "/internal/agent/activation", request)
            .await?;
        Ok(())
    }

    async fn update_run_usage(&self, request: &AgentWorkerUsageRequest) -> Result<(), AppError> {
        let _: Value = self
            .request_json(Method::POST, "/internal/agent/usage", request)
            .await?;
        Ok(())
    }

    async fn wait_run(&self, request: &AgentWorkerRunWaitRequest) -> Result<(), AppError> {
        let _: Value = self
            .request_json(Method::POST, "/internal/agent/wait-run", request)
            .await?;
        Ok(())
    }

    async fn update_context_summary(
        &self,
        request: &AgentWorkerContextSummaryRequest,
    ) -> Result<(), AppError> {
        let _: Value = self
            .request_json(Method::POST, "/internal/agent/context-summary", request)
            .await?;
        Ok(())
    }

    async fn worker_execution_by_key(
        &self,
        request: &AgentWorkerExecutionLookupRequest,
    ) -> Result<Option<AgentWorkerExecutionSnapshotWire>, AppError> {
        self.request_json(Method::POST, "/internal/agent/execution/lookup", request)
            .await
    }

    async fn create_prepared_execution(
        &self,
        request: &AgentWorkerExecutionCreateRequest,
    ) -> Result<Option<AgentWorkerExecutionSnapshotWire>, AppError> {
        self.request_json(Method::POST, "/internal/agent/execution/prepare", request)
            .await
    }

    async fn update_worker_execution(
        &self,
        request: &AgentWorkerExecutionUpdateRequest,
    ) -> Result<AgentWorkerExecutionSnapshotWire, AppError> {
        self.request_json(Method::POST, "/internal/agent/execution/update", request)
            .await
    }

    async fn read_execution_result(
        &self,
        request: &AgentWorkerExecutionResultRequest,
    ) -> Result<AgentExecutionResultPage, AppError> {
        self.request_json(Method::POST, "/internal/agent/execution/result", request)
            .await
    }

    async fn apply_message_request(
        &self,
        request: &AgentWorkerMessageRequest,
    ) -> Result<Option<AgentWorkerMessageSnapshotWire>, AppError> {
        self.request_json(Method::POST, "/internal/agent/message", request)
            .await
    }

    async fn worker_budget_snapshot(
        &self,
        request: &AgentWorkerBudgetRequest,
    ) -> Result<AgentWorkerBudgetSnapshotWire, AppError> {
        self.request_json(Method::POST, "/internal/agent/budget", request)
            .await
    }

    async fn fail_worker_run(&self, request: &AgentWorkerFailureRequest) -> Result<(), AppError> {
        let _: Value = self
            .request_json(Method::POST, "/internal/agent/failure", request)
            .await?;
        Ok(())
    }

    async fn result_refs_for_run(
        &self,
        request: &AgentWorkerResultRefsRequest,
    ) -> Result<AgentWorkerResultRefsSnapshotWire, AppError> {
        self.request_json(Method::POST, "/internal/agent/result-refs", request)
            .await
    }

    async fn complete_worker_run(
        &self,
        request: &AgentWorkerCompletionRequest,
    ) -> Result<AgentWorkerCompletionSnapshotWire, AppError> {
        self.request_json(Method::POST, "/internal/agent/completion", request)
            .await
    }

    async fn approved_execution(
        &self,
        request: &AgentWorkerApprovalCheckRequest,
    ) -> Result<AgentWorkerApprovalCheckSnapshotWire, AppError> {
        self.request_json(Method::POST, "/internal/agent/approval/check", request)
            .await
    }

    async fn request_approval(&self, request: &AgentWorkerApprovalRequest) -> Result<(), AppError> {
        let _: Value = self
            .request_json(Method::POST, "/internal/agent/approval/request", request)
            .await?;
        Ok(())
    }

    async fn review_delegation(
        &self,
        request: &AgentWorkerDelegationReviewRequest,
    ) -> Result<AgentWorkerDelegationReviewSnapshotWire, AppError> {
        self.request_json(Method::POST, "/internal/agent/delegation/review", request)
            .await
    }

    async fn ensure_canonical_subagent_thread(
        &self,
        request: &AgentWorkerDelegationThreadRequest,
    ) -> Result<AgentWorkerDelegationThreadSnapshotWire, AppError> {
        self.request_json(Method::POST, "/internal/agent/delegation/thread", request)
            .await
    }

    async fn append_worker_update(
        &self,
        request: &AgentWorkerEventRequest,
    ) -> Result<(), AppError> {
        let _: Value = self
            .request_json(Method::POST, "/internal/agent/event", request)
            .await?;
        Ok(())
    }

    async fn create_child_run(
        &self,
        request: &AgentWorkerDelegationStartRequest,
    ) -> Result<AgentWorkerDelegationStartSnapshotWire, AppError> {
        self.request_json(Method::POST, "/internal/agent/delegation/start", request)
            .await
    }

    async fn run_is_terminal(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<AgentWorkerRunStatusSnapshotWire, AppError> {
        self.request_json(Method::POST, "/internal/agent/run-status", request)
            .await
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
        wait: &AgentExternalTaskWaitSnapshot,
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
    ) -> Result<ModelCapabilities, AppError> {
        self.request_json(
            Method::POST,
            "/internal/agent/llm/capabilities",
            &AgentWorkerLlmCapabilitiesRequest {
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
    ) -> Result<AgentChatStream, AppError> {
        let request = request.snapshot_value()?;
        self.request_stream(&AgentWorkerChatRequest {
            user_id,
            snapshot: snapshot.cloned(),
            config_id,
            request,
        })
        .await
    }

    async fn project_editor_draft(
        &self,
        plan: &ChapterVisualPlanV2,
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
