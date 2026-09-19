use crate::AppState;
use a4n_agent_runtime::{AgentChatStream, AgentRuntimeHost};
use a4n_agent_runtime_contract::{AgentExternalTaskWaitSnapshot, AgentLlmCapabilities};
use a4n_api_contract::agent::{
    AgentExecutionResultPage, AgentSkillContent, AgentSkillSearchResult, AgentToolDescriptor,
    AgentToolSearchResult,
};
use a4n_api_contract::chapter_visual_plan::{
    ChapterVisualPlanV3, VisualPlanProjection, VisualPlanReadView,
};
use a4n_api_contract::runtime_config::RuntimeConfigSnapshot;
use a4n_error::AppError;
use a4n_llm_provider_contract::ChatRequest;
use a4n_worker_contract::*;
use async_trait::async_trait;
use serde_json::Value;

pub(crate) struct NativeAgentRuntimeHost {
    state: AppState,
}

impl NativeAgentRuntimeHost {
    pub(crate) fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl AgentRuntimeHost for NativeAgentRuntimeHost {
    async fn claim_worker(
        &self,
        request: &AgentWorkerClaimRequest,
    ) -> Result<Option<WorkerRequest<AgentRunPayload>>, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let Some(claimed) = self
            .state
            .agent_worker_claim_application()
            .claim_worker_run(&request.worker_id)
            .await?
        else {
            return Ok(None);
        };
        let snapshot = self.state.agent_runtime_snapshot(claimed.run.id).await;
        crate::actions::agent_runtime::worker_request_for_claimed_run(
            &claimed,
            &request.worker_id,
            snapshot,
        )
        .map(Some)
    }

    async fn load_run_context(
        &self,
        request: &WorkerRequest<AgentRunPayload>,
    ) -> Result<AgentWorkerRunContextWire, AppError> {
        crate::actions::agent_runtime::load_agent_run_context(&self.state, request).await
    }

    async fn renew_worker_lease(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<(), AppError> {
        crate::actions::agent_runtime::renew_agent_worker_lease(&self.state, request).await
    }

    async fn mark_turn_active(&self, request: &AgentWorkerRunLeaseRequest) -> Result<(), AppError> {
        crate::actions::agent_runtime::mark_agent_turn_active(&self.state, request).await
    }

    async fn update_run_usage(&self, request: &AgentWorkerUsageRequest) -> Result<(), AppError> {
        crate::actions::agent_runtime::update_agent_worker_usage(&self.state, request).await
    }

    async fn wait_run(&self, request: &AgentWorkerRunWaitRequest) -> Result<(), AppError> {
        crate::actions::agent_runtime::wait_agent_worker_run(&self.state, request).await
    }

    async fn update_context_summary(
        &self,
        request: &AgentWorkerContextSummaryRequest,
    ) -> Result<(), AppError> {
        crate::actions::agent_runtime::update_agent_context_summary(&self.state, request).await
    }

    async fn worker_execution_by_key(
        &self,
        request: &AgentWorkerExecutionLookupRequest,
    ) -> Result<Option<AgentWorkerExecutionSnapshotWire>, AppError> {
        crate::actions::agent_runtime::worker_execution_by_key(&self.state, request).await
    }

    async fn create_prepared_execution(
        &self,
        request: &AgentWorkerExecutionCreateRequest,
    ) -> Result<Option<AgentWorkerExecutionSnapshotWire>, AppError> {
        crate::actions::agent_runtime::create_prepared_execution(&self.state, request).await
    }

    async fn update_worker_execution(
        &self,
        request: &AgentWorkerExecutionUpdateRequest,
    ) -> Result<AgentWorkerExecutionSnapshotWire, AppError> {
        crate::actions::agent_runtime::update_worker_execution(&self.state, request).await
    }

    async fn read_execution_result(
        &self,
        request: &AgentWorkerExecutionResultRequest,
    ) -> Result<AgentExecutionResultPage, AppError> {
        crate::actions::agent_runtime::read_execution_result(&self.state, request).await
    }

    async fn read_memory(
        &self,
        request: &AgentWorkerMemoryRequest,
    ) -> Result<a4n_api_contract::agent::AgentMemorySearchResult, AppError> {
        crate::actions::agent_runtime::read_memory(&self.state, request).await
    }

    async fn search_agent_tools(
        &self,
        request: &AgentWorkerToolSearchRequest,
    ) -> Result<AgentToolSearchResult, AppError> {
        crate::actions::agent_runtime::search_agent_tools_for_worker(&self.state, request)
    }

    async fn describe_agent_tool(
        &self,
        request: &AgentWorkerToolDescribeRequest,
    ) -> Result<AgentToolDescriptor, AppError> {
        crate::actions::agent_runtime::describe_agent_tool_for_worker(&self.state, request)
    }

    async fn search_agent_skills(
        &self,
        request: &AgentWorkerSkillSearchRequest,
    ) -> Result<AgentSkillSearchResult, AppError> {
        crate::actions::agent_runtime::search_agent_skills_for_worker(&self.state, request).await
    }

    async fn load_agent_skill(
        &self,
        request: &AgentWorkerSkillLoadRequest,
    ) -> Result<AgentSkillContent, AppError> {
        crate::actions::agent_runtime::load_agent_skill_for_worker(&self.state, request).await
    }

    async fn verify_agent_chapters(
        &self,
        request: &AgentWorkerChapterScopeRequest,
    ) -> Result<(), AppError> {
        crate::actions::agent_runtime::verify_agent_chapters_for_worker(&self.state, request).await
    }

    async fn read_worker_history(
        &self,
        request: &AgentWorkerHistoryRequest,
    ) -> Result<AgentWorkerHistorySnapshotWire, AppError> {
        crate::actions::agent_runtime::read_worker_history(&self.state, request).await
    }

    async fn mark_recovery_required(
        &self,
        request: &AgentWorkerRecoveryRequest,
    ) -> Result<(), AppError> {
        crate::actions::agent_runtime::mark_agent_worker_recovery_required(&self.state, request)
            .await
    }

    async fn wait_for_editor(
        &self,
        request: &AgentWorkerEditorWaitRequest,
    ) -> Result<(), AppError> {
        crate::actions::agent_runtime::wait_for_agent_editor(&self.state, request).await
    }

    async fn expire_editor_waits(
        &self,
        request: &AgentWorkerEditorWaitExpiryRequest,
    ) -> Result<usize, AppError> {
        crate::actions::agent_runtime::expire_agent_editor_waits(&self.state, request).await
    }

    async fn expire_pending_approvals(
        &self,
        request: &AgentWorkerApprovalExpiryRequest,
    ) -> Result<usize, AppError> {
        crate::actions::agent_runtime::expire_agent_approvals(&self.state, request).await
    }

    async fn recover_expired_runs(
        &self,
        request: &AgentWorkerExpiredRunRecoveryRequest,
    ) -> Result<usize, AppError> {
        crate::actions::agent_runtime::recover_expired_agent_runs(&self.state, request).await
    }

    async fn generation_cancel_candidates(
        &self,
        request: &AgentWorkerGenerationCancelScanRequest,
    ) -> Result<Vec<AgentWorkerGenerationCancelSnapshotWire>, AppError> {
        crate::actions::agent_runtime::list_agent_generation_cancel_candidates(&self.state, request)
            .await
    }

    async fn workflow_cancel_candidates(
        &self,
        request: &AgentWorkerWorkflowCancelScanRequest,
    ) -> Result<Vec<AgentWorkerWorkflowCancelSnapshotWire>, AppError> {
        crate::actions::agent_runtime::list_agent_workflow_cancel_candidates(&self.state, request)
            .await
    }

    async fn external_task_waits(
        &self,
        request: &AgentWorkerExternalTaskWaitScanRequest,
    ) -> Result<Vec<AgentWorkerExternalTaskWaitSnapshotWire>, AppError> {
        crate::actions::agent_runtime::list_agent_external_task_waits(&self.state, request).await
    }

    async fn wake_external_task_wait(
        &self,
        request: &AgentWorkerExternalTaskWaitWakeRequest,
    ) -> Result<(), AppError> {
        crate::actions::agent_runtime::wake_agent_external_task_wait(&self.state, request).await
    }

    async fn claim_inbox(
        &self,
        request: &AgentWorkerInboxClaimRequest,
    ) -> Result<Option<AgentWorkerInboxSnapshotWire>, AppError> {
        crate::actions::agent_runtime::claim_agent_inbox(&self.state, request).await
    }

    async fn consume_inbox(
        &self,
        request: &AgentWorkerInboxConsumeRequest,
    ) -> Result<AgentWorkerInboxSnapshotWire, AppError> {
        crate::actions::agent_runtime::consume_agent_inbox(&self.state, request).await
    }

    async fn persist_inbox(
        &self,
        request: &AgentWorkerInboxPersistRequest,
    ) -> Result<AgentWorkerInboxPersistSnapshotWire, AppError> {
        crate::actions::agent_runtime::persist_agent_inbox(&self.state, request).await
    }

    async fn load_inbox_message(
        &self,
        request: &AgentWorkerInboxMessageRequest,
    ) -> Result<AgentWorkerMessageSnapshotWire, AppError> {
        crate::actions::agent_runtime::load_agent_inbox_message(&self.state, request).await
    }

    async fn apply_message_request(
        &self,
        request: &AgentWorkerMessageRequest,
    ) -> Result<Option<AgentWorkerMessageSnapshotWire>, AppError> {
        crate::actions::agent_runtime::apply_message_request(&self.state, request).await
    }

    async fn worker_budget_snapshot(
        &self,
        request: &AgentWorkerBudgetRequest,
    ) -> Result<AgentWorkerBudgetSnapshotWire, AppError> {
        crate::actions::agent_runtime::worker_budget_snapshot(&self.state, request).await
    }

    async fn fail_worker_run(&self, request: &AgentWorkerFailureRequest) -> Result<(), AppError> {
        crate::actions::agent_runtime::fail_agent_worker_run(&self.state, request).await
    }

    async fn result_refs_for_run(
        &self,
        request: &AgentWorkerResultRefsRequest,
    ) -> Result<AgentWorkerResultRefsSnapshotWire, AppError> {
        crate::actions::agent_runtime::result_refs_for_agent_worker_run(&self.state, request).await
    }

    async fn complete_worker_run(
        &self,
        request: &AgentWorkerCompletionRequest,
    ) -> Result<AgentWorkerCompletionSnapshotWire, AppError> {
        crate::actions::agent_runtime::complete_agent_worker_run(&self.state, request).await
    }

    async fn approved_execution(
        &self,
        request: &AgentWorkerApprovalCheckRequest,
    ) -> Result<AgentWorkerApprovalCheckSnapshotWire, AppError> {
        crate::actions::agent_runtime::check_agent_worker_approval(&self.state, request).await
    }

    async fn request_approval(&self, request: &AgentWorkerApprovalRequest) -> Result<(), AppError> {
        crate::actions::agent_runtime::request_agent_worker_approval(&self.state, request).await
    }

    async fn review_delegation(
        &self,
        request: &AgentWorkerDelegationReviewRequest,
    ) -> Result<AgentWorkerDelegationReviewSnapshotWire, AppError> {
        crate::actions::agent_runtime::review_agent_delegation(&self.state, request).await
    }

    async fn inherit_agent_delegation_scope(
        &self,
        request: &AgentWorkerDelegationScopeRequest,
    ) -> Result<AgentWorkerDelegationScopeSnapshotWire, AppError> {
        crate::actions::agent_runtime::inherit_agent_delegation_scope(&self.state, request).await
    }

    async fn ensure_canonical_subagent_thread(
        &self,
        request: &AgentWorkerDelegationThreadRequest,
    ) -> Result<AgentWorkerDelegationThreadSnapshotWire, AppError> {
        crate::actions::agent_runtime::ensure_agent_canonical_subagent_thread(&self.state, request)
            .await
    }

    async fn append_worker_update(
        &self,
        request: &AgentWorkerEventRequest,
    ) -> Result<(), AppError> {
        crate::actions::agent_runtime::append_agent_worker_update(&self.state, request).await
    }

    async fn create_child_run(
        &self,
        request: &AgentWorkerDelegationStartRequest,
    ) -> Result<AgentWorkerDelegationStartSnapshotWire, AppError> {
        crate::actions::agent_runtime::create_agent_child_run(&self.state, request).await
    }

    async fn run_is_terminal(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<AgentWorkerRunStatusSnapshotWire, AppError> {
        crate::actions::agent_runtime::agent_worker_run_is_terminal(&self.state, request).await
    }

    async fn execute_command(
        &self,
        user_id: i64,
        command: String,
        arguments: Value,
        request: WorkerRequest<AgentHostCommandPayload>,
    ) -> Result<Value, AppError> {
        let mut request = request;
        request.payload.command = command;
        request.payload.arguments = arguments;
        self.state
            .dispatch_agent_host_command_request(user_id, request)
            .await
    }

    async fn package_execution_enabled(&self) -> Result<bool, AppError> {
        crate::actions::agent_runtime::agent_package_execution_enabled(&self.state).await
    }

    async fn cancel_generation_task(
        &self,
        user_id: i64,
        task_id: String,
        attempt: u32,
    ) -> Result<(), AppError> {
        crate::actions::agent_runtime::cancel_agent_owned_generation_task(
            &self.state,
            user_id,
            &task_id,
            attempt,
        )
        .await
    }

    async fn cancel_workflow(&self, workflow_run_id: i64, reason: &str) -> Result<(), AppError> {
        crate::actions::agent_runtime::cancel_agent_owned_workflow(
            &self.state,
            workflow_run_id,
            reason,
        )
        .await
    }

    async fn read_external_task_wait(
        &self,
        wait: &AgentWorkerExternalTaskWaitSnapshotWire,
        snapshot: Option<&RuntimeConfigSnapshot>,
    ) -> Result<Value, AppError> {
        crate::actions::agent_runtime::read_external_task_wait(
            &self.state,
            AgentExternalTaskWaitSnapshot {
                run_id: wait.run_id,
                owner_user_id: wait.owner_user_id,
                scope: wait.scope.clone(),
                execution_id: wait.execution_id,
                tool_name: wait.tool_name.clone(),
                arguments_json: wait.arguments_json.clone(),
                duration_exhausted: wait.duration_exhausted,
            },
            snapshot.cloned(),
        )
        .await
    }

    async fn llm_capabilities(
        &self,
        user_id: i64,
        snapshot: Option<&RuntimeConfigSnapshot>,
        config_id: Option<i64>,
    ) -> Result<AgentLlmCapabilities, AppError> {
        crate::actions::agent_runtime::agent_llm_capabilities(
            &self.state,
            user_id,
            snapshot.cloned(),
            config_id,
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
        crate::actions::agent_runtime::agent_chat_stream(
            &self.state,
            user_id,
            snapshot.cloned(),
            config_id,
            request,
        )
        .await
    }

    async fn prepare_chat_request(
        &self,
        user_id: i64,
        snapshot: Option<&RuntimeConfigSnapshot>,
        config_id: Option<i64>,
        request: ChatRequest,
    ) -> Result<ChatRequest, AppError> {
        crate::actions::agent_runtime::prepare_agent_chat_request(
            &self.state,
            user_id,
            snapshot.cloned(),
            config_id,
            request,
        )
        .await
    }

    async fn project_editor_draft(
        &self,
        plan: &ChapterVisualPlanV3,
        view: &VisualPlanReadView,
    ) -> Result<VisualPlanProjection, AppError> {
        crate::actions::agent_runtime::project_agent_editor_draft(&self.state, plan, view)
    }
}
