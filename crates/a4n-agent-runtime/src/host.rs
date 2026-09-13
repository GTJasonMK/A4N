use a4n_agent_application::agent::{
    AgentExternalTaskWaitSnapshot, AgentRunContext, AgentWorkflowCancelSnapshot,
};
use a4n_api_contract::agent::AgentExecutionResultPage;
use a4n_api_contract::chapter_visual_plan::{
    ChapterVisualPlanV2, VisualPlanProjection, VisualPlanReadView,
};
use a4n_api_contract::runtime_config::RuntimeConfigSnapshot;
use a4n_error::AppError;
use a4n_llm::provider::{ChatChunk, ChatRequest, ModelCapabilities};
use a4n_worker_contract::{
    AgentRunPayload, AgentWorkerApprovalCheckRequest, AgentWorkerApprovalCheckSnapshotWire,
    AgentWorkerApprovalRequest, AgentWorkerBudgetRequest, AgentWorkerBudgetSnapshotWire,
    AgentWorkerClaimRequest, AgentWorkerCompletionRequest, AgentWorkerCompletionSnapshotWire,
    AgentWorkerContextSummaryRequest, AgentWorkerDelegationReviewRequest,
    AgentWorkerDelegationReviewSnapshotWire, AgentWorkerDelegationStartRequest,
    AgentWorkerDelegationStartSnapshotWire, AgentWorkerDelegationThreadRequest,
    AgentWorkerDelegationThreadSnapshotWire, AgentWorkerEventRequest,
    AgentWorkerExecutionCreateRequest, AgentWorkerExecutionLookupRequest,
    AgentWorkerExecutionResultRequest, AgentWorkerExecutionSnapshotWire,
    AgentWorkerExecutionUpdateRequest, AgentWorkerFailureRequest, AgentWorkerHistoryRequest,
    AgentWorkerHistorySnapshotWire, AgentWorkerInboxClaimRequest, AgentWorkerInboxConsumeRequest,
    AgentWorkerInboxMessageRequest, AgentWorkerInboxPersistRequest,
    AgentWorkerInboxPersistSnapshotWire, AgentWorkerInboxSnapshotWire, AgentWorkerMemoryRequest,
    AgentWorkerMessageRequest, AgentWorkerMessageSnapshotWire, AgentWorkerResultRefsRequest,
    AgentWorkerResultRefsSnapshotWire, AgentWorkerRunLeaseRequest,
    AgentWorkerRunStatusSnapshotWire, AgentWorkerRunWaitRequest, AgentWorkerUsageRequest,
    WorkerRequest,
};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

pub type AgentChatStream = Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>;

/// Boundary owned by the Host around the narrow Agent execution runtime.
///
/// The worker owns durable Agent state and the model loop. Project, Canvas,
/// workflow, package and provider-specific behavior stays behind this port.
#[async_trait]
pub trait AgentRuntimeHost: Send + Sync {
    async fn claim_worker(
        &self,
        request: &AgentWorkerClaimRequest,
    ) -> Result<Option<WorkerRequest<AgentRunPayload>>, AppError>;

    async fn load_run_context(
        &self,
        request: &WorkerRequest<AgentRunPayload>,
    ) -> Result<AgentRunContext, AppError>;

    async fn renew_worker_lease(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<(), AppError>;

    async fn mark_turn_active(&self, request: &AgentWorkerRunLeaseRequest) -> Result<(), AppError>;

    async fn update_run_usage(&self, request: &AgentWorkerUsageRequest) -> Result<(), AppError>;

    async fn wait_run(&self, request: &AgentWorkerRunWaitRequest) -> Result<(), AppError>;

    async fn update_context_summary(
        &self,
        request: &AgentWorkerContextSummaryRequest,
    ) -> Result<(), AppError>;

    async fn worker_execution_by_key(
        &self,
        request: &AgentWorkerExecutionLookupRequest,
    ) -> Result<Option<AgentWorkerExecutionSnapshotWire>, AppError>;

    async fn create_prepared_execution(
        &self,
        request: &AgentWorkerExecutionCreateRequest,
    ) -> Result<Option<AgentWorkerExecutionSnapshotWire>, AppError>;

    async fn update_worker_execution(
        &self,
        request: &AgentWorkerExecutionUpdateRequest,
    ) -> Result<AgentWorkerExecutionSnapshotWire, AppError>;

    async fn read_execution_result(
        &self,
        request: &AgentWorkerExecutionResultRequest,
    ) -> Result<AgentExecutionResultPage, AppError>;

    async fn read_memory(
        &self,
        request: &AgentWorkerMemoryRequest,
    ) -> Result<a4n_api_contract::agent::AgentMemorySearchResult, AppError>;

    async fn read_worker_history(
        &self,
        request: &AgentWorkerHistoryRequest,
    ) -> Result<AgentWorkerHistorySnapshotWire, AppError>;

    async fn claim_inbox(
        &self,
        request: &AgentWorkerInboxClaimRequest,
    ) -> Result<Option<AgentWorkerInboxSnapshotWire>, AppError>;

    async fn consume_inbox(
        &self,
        request: &AgentWorkerInboxConsumeRequest,
    ) -> Result<AgentWorkerInboxSnapshotWire, AppError>;

    async fn persist_inbox(
        &self,
        request: &AgentWorkerInboxPersistRequest,
    ) -> Result<AgentWorkerInboxPersistSnapshotWire, AppError>;

    async fn load_inbox_message(
        &self,
        request: &AgentWorkerInboxMessageRequest,
    ) -> Result<AgentWorkerMessageSnapshotWire, AppError>;

    async fn apply_message_request(
        &self,
        request: &AgentWorkerMessageRequest,
    ) -> Result<Option<AgentWorkerMessageSnapshotWire>, AppError>;

    async fn worker_budget_snapshot(
        &self,
        request: &AgentWorkerBudgetRequest,
    ) -> Result<AgentWorkerBudgetSnapshotWire, AppError>;

    async fn fail_worker_run(&self, request: &AgentWorkerFailureRequest) -> Result<(), AppError>;

    async fn result_refs_for_run(
        &self,
        request: &AgentWorkerResultRefsRequest,
    ) -> Result<AgentWorkerResultRefsSnapshotWire, AppError>;

    async fn complete_worker_run(
        &self,
        request: &AgentWorkerCompletionRequest,
    ) -> Result<AgentWorkerCompletionSnapshotWire, AppError>;

    async fn approved_execution(
        &self,
        request: &AgentWorkerApprovalCheckRequest,
    ) -> Result<AgentWorkerApprovalCheckSnapshotWire, AppError>;

    async fn request_approval(&self, request: &AgentWorkerApprovalRequest) -> Result<(), AppError>;

    async fn review_delegation(
        &self,
        request: &AgentWorkerDelegationReviewRequest,
    ) -> Result<AgentWorkerDelegationReviewSnapshotWire, AppError>;

    async fn ensure_canonical_subagent_thread(
        &self,
        request: &AgentWorkerDelegationThreadRequest,
    ) -> Result<AgentWorkerDelegationThreadSnapshotWire, AppError>;

    async fn append_worker_update(&self, request: &AgentWorkerEventRequest)
    -> Result<(), AppError>;

    async fn create_child_run(
        &self,
        request: &AgentWorkerDelegationStartRequest,
    ) -> Result<AgentWorkerDelegationStartSnapshotWire, AppError>;

    async fn run_is_terminal(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<AgentWorkerRunStatusSnapshotWire, AppError>;

    async fn execute_command(
        &self,
        user_id: i64,
        command: String,
        arguments: Value,
        request: WorkerRequest<a4n_worker_contract::AgentHostCommandPayload>,
    ) -> Result<Value, AppError>;

    async fn package_execution_enabled(&self) -> Result<bool, AppError>;

    async fn cancel_generation_task(
        &self,
        user_id: i64,
        task_id: String,
        attempt: u32,
    ) -> Result<(), AppError>;

    async fn cancel_workflow(&self, workflow_run_id: i64, reason: &str) -> Result<(), AppError>;

    async fn read_external_task_wait(
        &self,
        wait: &AgentExternalTaskWaitSnapshot,
        snapshot: Option<&RuntimeConfigSnapshot>,
    ) -> Result<Value, AppError>;

    async fn llm_capabilities(
        &self,
        user_id: i64,
        snapshot: Option<&RuntimeConfigSnapshot>,
        config_id: Option<i64>,
    ) -> Result<ModelCapabilities, AppError>;

    async fn chat_stream(
        &self,
        user_id: i64,
        snapshot: Option<&RuntimeConfigSnapshot>,
        config_id: Option<i64>,
        request: ChatRequest,
    ) -> Result<AgentChatStream, AppError>;

    async fn project_editor_draft(
        &self,
        plan: &ChapterVisualPlanV2,
        view: &VisualPlanReadView,
    ) -> Result<VisualPlanProjection, AppError>;

    async fn notify_external_task_wait(
        &self,
        _context: &AgentRunContext,
        _value: &Value,
    ) -> Result<(), AppError> {
        Ok(())
    }

    async fn cancel_stopped_workflows(
        &self,
        workflows: &[AgentWorkflowCancelSnapshot],
    ) -> Result<(), AppError> {
        for workflow in workflows.iter().filter(|workflow| workflow.owner_stopped) {
            self.cancel_workflow(workflow.workflow_run_id, "所属 Agent 或祖先任务已停止")
                .await?;
        }
        Ok(())
    }
}
