use super::*;

#[async_trait::async_trait]
pub trait AgentApprovalExpiryApplicationPort: Send + Sync {
    async fn expire_pending_approvals(&self, limit: u64) -> Result<usize, AppError>;
}

#[async_trait::async_trait]
pub trait AgentEditorClaimApplicationPort: Send + Sync {
    async fn claim_editor_request(
        &self,
        user_id: i64,
        input: &RespondAgentEditorInput,
    ) -> Result<(AgentWorkerRun, AgentEditorRequest), AppError>;
}

#[async_trait::async_trait]
pub trait AgentEditorResponseApplicationPort: Send + Sync {
    async fn wait_for_editor(
        &self,
        claimed: &AgentWorkerRun,
        request: &AgentEditorRequest,
    ) -> Result<(), AppError>;

    async fn wait_for_editor_request(
        &self,
        request: &AgentWorkerEditorWaitRequest,
    ) -> Result<(), AppError>;

    async fn requeue_after_external_response(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<(), AppError>;
}

#[async_trait::async_trait]
pub trait AgentEditorWaitApplicationPort: Send + Sync {
    async fn expire_editor_requests(&self, limit: u64) -> Result<usize, AppError>;
}

#[async_trait::async_trait]
pub trait AgentExternalTaskWaitApplicationPort: Send + Sync {
    async fn external_task_waits(
        &self,
        after_run_id: i64,
    ) -> Result<Vec<AgentExternalTaskWaitSnapshot>, AppError>;

    async fn wake_external_task_wait(&self, run_id: i64, execution_id: i64)
    -> Result<(), AppError>;
}

#[async_trait::async_trait]
pub trait AgentGenerationCancelApplicationPort: Send + Sync {
    async fn generation_cancel_candidates(
        &self,
        after_task_id: &str,
    ) -> Result<Vec<AgentGenerationCancelSnapshot>, AppError>;
}

#[async_trait::async_trait]
pub trait AgentRunLeaseRecoveryApplicationPort: Send + Sync {
    async fn recover_expired_runs(
        &self,
        limit: u64,
        error_message: Option<&str>,
    ) -> Result<usize, AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkerHostCommandApplicationPort: Send + Sync {
    async fn authorize_worker_host_command(
        &self,
        request: &WorkerRequest<AgentHostCommandPayload>,
    ) -> Result<i64, AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkerActivationApplicationPort: Send + Sync {
    async fn mark_turn_active(&self, claimed: &AgentWorkerRun) -> Result<(), AppError>;

    async fn mark_turn_active_request(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<(), AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkerApprovalApplicationPort: Send + Sync {
    async fn approved_execution(&self, thread_id: i64, execution_id: i64)
    -> Result<bool, AppError>;

    async fn approved_execution_request(
        &self,
        request: &AgentWorkerApprovalCheckRequest,
    ) -> Result<AgentWorkerApprovalCheckSnapshotWire, AppError>;

    async fn request_worker_approval(
        &self,
        claimed: &AgentWorkerRun,
        execution_id: i64,
        operation_json: Value,
        resource_preconditions_json: Option<Value>,
        expires_at: Option<NaiveDateTime>,
    ) -> Result<(), AppError>;

    async fn request_worker_approval_request(
        &self,
        request: &AgentWorkerApprovalRequest,
    ) -> Result<(), AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkerBudgetApplicationPort: Send + Sync {
    async fn worker_budget_snapshot(
        &self,
        run_id: i64,
    ) -> Result<AgentWorkerBudgetSnapshot, AppError>;

    async fn worker_budget_snapshot_request(
        &self,
        request: &AgentWorkerBudgetRequest,
    ) -> Result<AgentWorkerBudgetSnapshot, AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkerClaimApplicationPort: Send + Sync {
    async fn claim_worker_run(&self, worker_id: &str) -> Result<Option<AgentWorkerRun>, AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkerCompletionApplicationPort: Send + Sync {
    async fn run_is_terminal(&self, claimed: &AgentWorkerRun) -> Result<bool, AppError>;

    async fn result_refs_for_run(&self, claimed: &AgentWorkerRun) -> Result<Value, AppError>;

    async fn result_refs_for_run_request(
        &self,
        request: &AgentWorkerResultRefsRequest,
    ) -> Result<AgentWorkerResultRefsSnapshotWire, AppError>;

    async fn complete_worker_run_request(
        &self,
        request: &AgentWorkerCompletionRequest,
    ) -> Result<AgentWorkerCompletionSnapshotWire, AppError>;

    async fn run_is_terminal_request(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<AgentWorkerRunStatusSnapshotWire, AppError>;

    async fn complete_child_run(
        &self,
        claimed: &AgentWorkerRun,
        content: String,
        result_summary: String,
        result_refs_json: Option<Value>,
        observed_input_message_id: i64,
    ) -> Result<bool, AppError>;

    async fn complete_claimed_run(
        &self,
        claimed: &AgentWorkerRun,
        content: String,
        result_refs_json: Option<Value>,
        observed_input_message_id: i64,
    ) -> Result<bool, AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkerContextApplicationPort: Send + Sync {
    async fn load_run_context(&self, claimed: &AgentWorkerRun)
    -> Result<AgentRunContext, AppError>;

    async fn update_context_summary(
        &self,
        claimed: &AgentWorkerRun,
        context_summary: Value,
    ) -> Result<(), AppError>;

    async fn update_context_summary_request(
        &self,
        request: &AgentWorkerContextSummaryRequest,
    ) -> Result<(), AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkerDelegationApplicationPort: Send + Sync {
    async fn review_delegation(
        &self,
        user_id: i64,
        input: ReviewAgentDelegationInput,
        parent_run_id: Option<i64>,
    ) -> Result<AgentDelegationView, AppError>;

    async fn review_delegation_request(
        &self,
        request: &AgentWorkerDelegationReviewRequest,
    ) -> Result<AgentWorkerDelegationReviewSnapshotWire, AppError>;

    async fn ensure_canonical_subagent_thread_request(
        &self,
        request: &AgentWorkerDelegationThreadRequest,
    ) -> Result<AgentWorkerDelegationThreadSnapshotWire, AppError>;

    async fn append_worker_update_request(
        &self,
        request: &AgentWorkerEventRequest,
    ) -> Result<(), AppError>;

    async fn create_child_run_request(
        &self,
        request: &AgentWorkerDelegationStartRequest,
    ) -> Result<AgentWorkerDelegationStartSnapshotWire, AppError>;

    fn inherit_subagent_allowlists(
        &self,
        parent: &AgentScope,
        child: &mut AgentScope,
    ) -> Result<(), AppError>;

    async fn ensure_canonical_subagent_thread(
        &self,
        user_id: i64,
        input: EnsureCanonicalSubagentThreadInput,
    ) -> Result<AgentThreadSnapshot, AppError>;

    async fn append_worker_update(
        &self,
        claimed: &AgentWorkerRun,
        kind: &str,
        status: &str,
        public_text: String,
        payload_json: Option<Value>,
    ) -> Result<(), AppError>;

    async fn create_child_run(
        &self,
        claimed: &AgentWorkerRun,
        input: WorkerDelegationInput,
    ) -> Result<WorkerDelegationResult, AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkerExecutionApplicationPort: Send + Sync {
    async fn worker_execution_by_key(
        &self,
        run_id: i64,
        key: &str,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError>;

    async fn create_prepared_execution(
        &self,
        input: WorkerExecutionInput,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError>;

    async fn update_worker_execution(
        &self,
        input: WorkerExecutionUpdate,
    ) -> Result<AgentExecutionSnapshot, AppError>;

    async fn worker_execution_by_key_request(
        &self,
        request: &AgentWorkerExecutionLookupRequest,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError>;

    async fn create_prepared_execution_request(
        &self,
        request: &AgentWorkerExecutionCreateRequest,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError>;

    async fn update_worker_execution_request(
        &self,
        request: &AgentWorkerExecutionUpdateRequest,
    ) -> Result<AgentExecutionSnapshot, AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkerFailureApplicationPort: Send + Sync {
    async fn fail_child_run(
        &self,
        claimed: &AgentWorkerRun,
        error_message: String,
    ) -> Result<(), AppError>;

    async fn fail_worker_run(&self, request: &AgentWorkerFailureRequest) -> Result<(), AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkerHistoryApplicationPort: Send + Sync {
    async fn user_messages_for_run(
        &self,
        run_id: i64,
    ) -> Result<Vec<AgentMessageSnapshot>, AppError>;

    async fn latest_model_input_message_id(&self, run_id: i64) -> Result<Option<i64>, AppError>;

    async fn worker_history_request(
        &self,
        request: &AgentWorkerHistoryRequest,
    ) -> Result<AgentWorkerHistorySnapshotWire, AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkerInboxApplicationPort: Send + Sync {
    async fn claim_inbox_for_thread(
        &self,
        thread_id: i64,
    ) -> Result<Option<AgentInboxSnapshot>, AppError>;

    async fn consume_claimed_inbox(
        &self,
        row: &AgentInboxSnapshot,
    ) -> Result<AgentInboxSnapshot, AppError>;

    async fn persist_claimed_inbox(
        &self,
        claimed: &AgentWorkerRun,
        row: &AgentInboxSnapshot,
    ) -> Result<(AgentInboxSnapshot, AgentMessageSnapshot), AppError>;

    async fn load_message_for_worker(
        &self,
        thread_id: i64,
        message_id: i64,
    ) -> Result<AgentMessageSnapshot, AppError>;

    async fn claim_inbox_request(
        &self,
        request: &AgentWorkerInboxClaimRequest,
    ) -> Result<Option<AgentWorkerInboxSnapshotWire>, AppError>;

    async fn consume_inbox_request(
        &self,
        request: &AgentWorkerInboxConsumeRequest,
    ) -> Result<AgentWorkerInboxSnapshotWire, AppError>;

    async fn persist_inbox_request(
        &self,
        request: &AgentWorkerInboxPersistRequest,
    ) -> Result<AgentWorkerInboxPersistSnapshotWire, AppError>;

    async fn load_message_request(
        &self,
        request: &AgentWorkerInboxMessageRequest,
    ) -> Result<AgentWorkerMessageSnapshotWire, AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkerLeaseApplicationPort: Send + Sync {
    async fn renew_worker_lease(&self, claimed: &AgentWorkerRun) -> Result<(), AppError>;

    async fn renew_worker_lease_request(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<(), AppError>;
}

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
pub trait AgentWorkerReadApplicationPort: Send + Sync {
    fn search_tools_for_worker(
        &self,
        input: SearchAgentToolsInput,
    ) -> Result<AgentToolSearchResult, AppError>;

    fn describe_tool_for_worker(
        &self,
        input: DescribeAgentToolInput,
    ) -> Result<AgentToolDescriptor, AppError>;

    async fn verify_chapters_in_project_for_worker(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_ids: &[i64],
    ) -> Result<(), AppError>;

    async fn read_memory_for_run(
        &self,
        context: &AgentRunContext,
        input: ReadAgentMemoryInput,
    ) -> Result<AgentMemorySearchResult, AppError>;

    async fn read_memory_request(
        &self,
        request: &AgentWorkerMemoryRequest,
    ) -> Result<AgentMemorySearchResult, AppError>;

    async fn read_execution_result_for_run(
        &self,
        context: &AgentRunContext,
        input: ReadAgentExecutionResultInput,
    ) -> Result<AgentExecutionResultPage, AppError>;

    async fn read_execution_result_request(
        &self,
        request: &AgentWorkerExecutionResultRequest,
    ) -> Result<AgentExecutionResultPage, AppError>;

    async fn search_skills_for_worker(
        &self,
        request: &AgentWorkerSkillSearchRequest,
    ) -> Result<AgentSkillSearchResult, AppError>;

    async fn load_skill_for_worker(
        &self,
        request: &AgentWorkerSkillLoadRequest,
    ) -> Result<AgentSkillContent, AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkerRecoveryApplicationPort: Send + Sync {
    async fn mark_recovery_required(
        &self,
        claimed: &AgentWorkerRun,
        reason: String,
    ) -> Result<(), AppError>;

    async fn mark_recovery_required_request(
        &self,
        request: &AgentWorkerRecoveryRequest,
    ) -> Result<(), AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkerUsageApplicationPort: Send + Sync {
    async fn update_run_usage(&self, input: WorkerUsage) -> Result<(), AppError>;

    async fn update_run_usage_request(
        &self,
        request: &AgentWorkerUsageRequest,
    ) -> Result<(), AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkerWaitApplicationPort: Send + Sync {
    async fn wait_run(&self, claimed: &AgentWorkerRun, reason: &str) -> Result<(), AppError>;

    async fn wait_run_request(&self, request: &AgentWorkerRunWaitRequest) -> Result<(), AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkflowBindingApplicationPort: Send + Sync {
    async fn bind_owned_workflow(
        &self,
        origin: &AgentWorkflowOrigin,
        workflow_id: i64,
    ) -> Result<(), AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkflowCancelApplicationPort: Send + Sync {
    async fn workflow_cancel_candidates(
        &self,
        after_id: i64,
    ) -> Result<Vec<AgentWorkflowCancelSnapshot>, AppError>;
}

pub trait AgentWorkerApplicationPort:
    AgentApprovalExpiryApplicationPort
    + AgentEditorClaimApplicationPort
    + AgentEditorResponseApplicationPort
    + AgentEditorWaitApplicationPort
    + AgentExternalTaskWaitApplicationPort
    + AgentGenerationCancelApplicationPort
    + AgentRunLeaseRecoveryApplicationPort
    + AgentWorkerActivationApplicationPort
    + AgentWorkerApprovalApplicationPort
    + AgentWorkerBudgetApplicationPort
    + AgentWorkerClaimApplicationPort
    + AgentWorkerCompletionApplicationPort
    + AgentWorkerContextApplicationPort
    + AgentWorkerDelegationApplicationPort
    + AgentWorkerExecutionApplicationPort
    + AgentWorkerFailureApplicationPort
    + AgentWorkerHistoryApplicationPort
    + AgentWorkerHostCommandApplicationPort
    + AgentWorkerInboxApplicationPort
    + AgentWorkerLeaseApplicationPort
    + AgentWorkerMessageApplicationPort
    + AgentWorkerReadApplicationPort
    + AgentWorkerRecoveryApplicationPort
    + AgentWorkerUsageApplicationPort
    + AgentWorkerWaitApplicationPort
    + AgentWorkflowBindingApplicationPort
    + AgentWorkflowCancelApplicationPort
    + Send
    + Sync
{
}

impl<T> AgentWorkerApplicationPort for T where
    T: AgentApprovalExpiryApplicationPort
        + AgentEditorClaimApplicationPort
        + AgentEditorResponseApplicationPort
        + AgentEditorWaitApplicationPort
        + AgentExternalTaskWaitApplicationPort
        + AgentGenerationCancelApplicationPort
        + AgentRunLeaseRecoveryApplicationPort
        + AgentWorkerActivationApplicationPort
        + AgentWorkerApprovalApplicationPort
        + AgentWorkerBudgetApplicationPort
        + AgentWorkerClaimApplicationPort
        + AgentWorkerCompletionApplicationPort
        + AgentWorkerContextApplicationPort
        + AgentWorkerDelegationApplicationPort
        + AgentWorkerExecutionApplicationPort
        + AgentWorkerFailureApplicationPort
        + AgentWorkerHistoryApplicationPort
        + AgentWorkerHostCommandApplicationPort
        + AgentWorkerInboxApplicationPort
        + AgentWorkerLeaseApplicationPort
        + AgentWorkerMessageApplicationPort
        + AgentWorkerReadApplicationPort
        + AgentWorkerRecoveryApplicationPort
        + AgentWorkerUsageApplicationPort
        + AgentWorkerWaitApplicationPort
        + AgentWorkflowBindingApplicationPort
        + AgentWorkflowCancelApplicationPort
        + Send
        + Sync
{
}
