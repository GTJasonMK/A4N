use super::*;

pub async fn claim_agent_worker_request(
    state: &AppState,
    worker_id: &str,
) -> Result<Option<WorkerEnvelope<AgentRunPayload>>, AppError> {
    if worker_id.trim().is_empty() {
        return Err(AppError::InvalidArgument("Agent worker_id 不能为空".into()));
    }
    let _admission = state.lock_agent_admission().await;
    if !crate::actions::package::package_execution_enabled(state, PackageId::Agent).await? {
        return Ok(None);
    }
    let Some(claimed) = state
        .agent_worker_claim_application()
        .claim_worker_run(worker_id)
        .await?
    else {
        return Ok(None);
    };
    let runtime_snapshot = state.agent_runtime_snapshot(claimed.run.id).await;
    match worker_request_for_claimed_run(&claimed, worker_id, runtime_snapshot) {
        Ok(request) => Ok(Some(request)),
        Err(error) => {
            let _ = state
                .agent_worker_failure_application()
                .fail_child_run(&claimed, error.to_string())
                .await;
            Err(error)
        }
    }
}

pub async fn load_agent_run_context(
    state: &AppState,
    request: &WorkerEnvelope<AgentRunPayload>,
) -> Result<AgentWorkerRunContextWire, AppError> {
    request.validate_agent_run().map_err(|error| {
        AppError::InvalidArgument(format!("Agent worker request 无效: {error}"))
    })?;
    let claimed = AgentWorkerRun::from_wire(request.payload.worker_run.clone())?;
    state
        .agent_worker_context_application()
        .load_run_context(&claimed)
        .await
        .map(|context| context.to_wire())
}

pub async fn renew_agent_worker_lease(
    state: &AppState,
    request: &AgentWorkerRunLeaseRequest,
) -> Result<(), AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent worker lease 无效: {error}")))?;
    state
        .agent_worker_lease_application()
        .renew_worker_lease_request(request)
        .await
}

pub async fn mark_agent_turn_active(
    state: &AppState,
    request: &AgentWorkerRunLeaseRequest,
) -> Result<(), AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent worker lease 无效: {error}")))?;
    state
        .agent_worker_activation_application()
        .mark_turn_active_request(request)
        .await
}

pub async fn update_agent_worker_usage(
    state: &AppState,
    request: &AgentWorkerUsageRequest,
) -> Result<(), AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent worker usage 无效: {error}")))?;
    state
        .agent_worker_usage_application()
        .update_run_usage_request(request)
        .await
}

pub async fn wait_agent_worker_run(
    state: &AppState,
    request: &AgentWorkerRunWaitRequest,
) -> Result<(), AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent worker wait 无效: {error}")))?;
    state
        .agent_worker_wait_application()
        .wait_run_request(request)
        .await
}

pub async fn update_agent_context_summary(
    state: &AppState,
    request: &AgentWorkerContextSummaryRequest,
) -> Result<(), AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent context summary 无效: {error}"))
    })?;
    state
        .agent_worker_context_application()
        .update_context_summary_request(request)
        .await
}

pub async fn worker_execution_by_key(
    state: &AppState,
    request: &AgentWorkerExecutionLookupRequest,
) -> Result<Option<AgentWorkerExecutionSnapshotWire>, AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent execution lookup 无效: {error}"))
    })?;
    state
        .agent_worker_execution_application()
        .worker_execution_by_key_request(request)
        .await
        .map(|snapshot| snapshot.map(Into::into))
}

pub async fn create_prepared_execution(
    state: &AppState,
    request: &AgentWorkerExecutionCreateRequest,
) -> Result<Option<AgentWorkerExecutionSnapshotWire>, AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent execution create 无效: {error}"))
    })?;
    state
        .agent_worker_execution_application()
        .create_prepared_execution_request(request)
        .await
        .map(|snapshot| snapshot.map(Into::into))
}

pub async fn update_worker_execution(
    state: &AppState,
    request: &AgentWorkerExecutionUpdateRequest,
) -> Result<AgentWorkerExecutionSnapshotWire, AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent execution update 无效: {error}"))
    })?;
    state
        .agent_worker_execution_application()
        .update_worker_execution_request(request)
        .await
        .map(Into::into)
}

pub async fn read_execution_result(
    state: &AppState,
    request: &AgentWorkerExecutionResultRequest,
) -> Result<a4n_api_contract::agent::AgentExecutionResultPage, AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent execution result 无效: {error}"))
    })?;
    state
        .agent_worker_read_application()
        .read_execution_result_request(request)
        .await
}

pub async fn read_memory(
    state: &AppState,
    request: &AgentWorkerMemoryRequest,
) -> Result<a4n_api_contract::agent::AgentMemorySearchResult, AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent memory 无效: {error}")))?;
    state
        .agent_worker_read_application()
        .read_memory_request(request)
        .await
}

pub fn search_agent_tools_for_worker(
    state: &AppState,
    request: &AgentWorkerToolSearchRequest,
) -> Result<AgentToolSearchResult, AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent tool search 无效: {error}")))?;
    state
        .agent_worker_read_application()
        .search_tools_for_worker(request.input.clone())
}

pub fn describe_agent_tool_for_worker(
    state: &AppState,
    request: &AgentWorkerToolDescribeRequest,
) -> Result<AgentToolDescriptor, AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent tool describe 无效: {error}")))?;
    state
        .agent_worker_read_application()
        .describe_tool_for_worker(request.input.clone())
}

pub async fn verify_agent_chapters_for_worker(
    state: &AppState,
    request: &AgentWorkerChapterScopeRequest,
) -> Result<(), AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent chapter scope 无效: {error}")))?;
    state
        .agent_worker_read_application()
        .verify_chapters_in_project_for_worker(
            request.user_id,
            request.project_id,
            &request.chapter_ids,
        )
        .await
}

pub async fn read_worker_history(
    state: &AppState,
    request: &AgentWorkerHistoryRequest,
) -> Result<AgentWorkerHistorySnapshotWire, AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent history 无效: {error}")))?;
    state
        .agent_worker_history_application()
        .worker_history_request(request)
        .await
}

pub async fn mark_agent_worker_recovery_required(
    state: &AppState,
    request: &AgentWorkerRecoveryRequest,
) -> Result<(), AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent recovery 无效: {error}")))?;
    state
        .agent_worker_recovery_application()
        .mark_recovery_required_request(request)
        .await
}

pub async fn wait_for_agent_editor(
    state: &AppState,
    request: &AgentWorkerEditorWaitRequest,
) -> Result<(), AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent editor wait 无效: {error}")))?;
    state
        .agent_editor_response_application()
        .wait_for_editor_request(request)
        .await
}

pub async fn expire_agent_editor_waits(
    state: &AppState,
    request: &AgentWorkerEditorWaitExpiryRequest,
) -> Result<usize, AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent editor wait expiry 无效: {error}"))
    })?;
    state
        .agent_editor_wait_application()
        .expire_editor_requests(request.limit)
        .await
}

pub async fn expire_agent_approvals(
    state: &AppState,
    request: &AgentWorkerApprovalExpiryRequest,
) -> Result<usize, AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent approval expiry 无效: {error}"))
    })?;
    state
        .agent_approval_expiry_application()
        .expire_pending_approvals(request.limit)
        .await
}

pub async fn recover_expired_agent_runs(
    state: &AppState,
    request: &AgentWorkerExpiredRunRecoveryRequest,
) -> Result<usize, AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent expired run recovery 无效: {error}"))
    })?;
    state
        .agent_run_lease_recovery_application()
        .recover_expired_runs(request.limit, request.error_message.as_deref())
        .await
}

pub async fn claim_agent_inbox(
    state: &AppState,
    request: &AgentWorkerInboxClaimRequest,
) -> Result<Option<AgentWorkerInboxSnapshotWire>, AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent inbox claim 无效: {error}")))?;
    state
        .agent_worker_inbox_application()
        .claim_inbox_request(request)
        .await
}

pub async fn consume_agent_inbox(
    state: &AppState,
    request: &AgentWorkerInboxConsumeRequest,
) -> Result<AgentWorkerInboxSnapshotWire, AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent inbox consume 无效: {error}")))?;
    state
        .agent_worker_inbox_application()
        .consume_inbox_request(request)
        .await
}

pub async fn persist_agent_inbox(
    state: &AppState,
    request: &AgentWorkerInboxPersistRequest,
) -> Result<AgentWorkerInboxPersistSnapshotWire, AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent inbox persist 无效: {error}")))?;
    state
        .agent_worker_inbox_application()
        .persist_inbox_request(request)
        .await
}

pub async fn load_agent_inbox_message(
    state: &AppState,
    request: &AgentWorkerInboxMessageRequest,
) -> Result<AgentWorkerMessageSnapshotWire, AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent inbox message 无效: {error}")))?;
    state
        .agent_worker_inbox_application()
        .load_message_request(request)
        .await
}

pub async fn apply_message_request(
    state: &AppState,
    request: &AgentWorkerMessageRequest,
) -> Result<Option<AgentWorkerMessageSnapshotWire>, AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent message 无效: {error}")))?;
    state
        .agent_worker_message_application()
        .apply_message_request(request)
        .await
        .map(|snapshot| snapshot.map(Into::into))
}

pub async fn worker_budget_snapshot(
    state: &AppState,
    request: &AgentWorkerBudgetRequest,
) -> Result<AgentWorkerBudgetSnapshotWire, AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent budget 无效: {error}")))?;
    state
        .agent_worker_budget_application()
        .worker_budget_snapshot_request(request)
        .await
        .map(Into::into)
}

pub async fn fail_agent_worker_run(
    state: &AppState,
    request: &AgentWorkerFailureRequest,
) -> Result<(), AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent failure 无效: {error}")))?;
    state
        .agent_worker_failure_application()
        .fail_worker_run(request)
        .await
}

pub async fn result_refs_for_agent_worker_run(
    state: &AppState,
    request: &AgentWorkerResultRefsRequest,
) -> Result<AgentWorkerResultRefsSnapshotWire, AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent result refs 无效: {error}")))?;
    state
        .agent_worker_completion_application()
        .result_refs_for_run_request(request)
        .await
}

pub async fn complete_agent_worker_run(
    state: &AppState,
    request: &AgentWorkerCompletionRequest,
) -> Result<AgentWorkerCompletionSnapshotWire, AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent completion 无效: {error}")))?;
    state
        .agent_worker_completion_application()
        .complete_worker_run_request(request)
        .await
}

pub async fn check_agent_worker_approval(
    state: &AppState,
    request: &AgentWorkerApprovalCheckRequest,
) -> Result<AgentWorkerApprovalCheckSnapshotWire, AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent approval check 无效: {error}"))
    })?;
    state
        .agent_worker_approval_application()
        .approved_execution_request(request)
        .await
}

pub async fn request_agent_worker_approval(
    state: &AppState,
    request: &AgentWorkerApprovalRequest,
) -> Result<(), AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent approval request 无效: {error}"))
    })?;
    state
        .agent_worker_approval_application()
        .request_worker_approval_request(request)
        .await
}

pub async fn review_agent_delegation(
    state: &AppState,
    request: &AgentWorkerDelegationReviewRequest,
) -> Result<AgentWorkerDelegationReviewSnapshotWire, AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent delegation review 无效: {error}"))
    })?;
    state
        .agent_worker_delegation_application()
        .review_delegation_request(request)
        .await
}

pub async fn inherit_agent_delegation_scope(
    state: &AppState,
    request: &AgentWorkerDelegationScopeRequest,
) -> Result<AgentWorkerDelegationScopeSnapshotWire, AppError> {
    let mut scope = request.child_scope.clone();
    state
        .agent_worker_delegation_application()
        .inherit_subagent_allowlists(&request.parent_scope, &mut scope)?;
    Ok(AgentWorkerDelegationScopeSnapshotWire { scope })
}

pub async fn ensure_agent_canonical_subagent_thread(
    state: &AppState,
    request: &AgentWorkerDelegationThreadRequest,
) -> Result<AgentWorkerDelegationThreadSnapshotWire, AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent delegation thread 无效: {error}"))
    })?;
    state
        .agent_worker_delegation_application()
        .ensure_canonical_subagent_thread_request(request)
        .await
}

pub async fn append_agent_worker_update(
    state: &AppState,
    request: &AgentWorkerEventRequest,
) -> Result<(), AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent worker event 无效: {error}")))?;
    state
        .agent_worker_delegation_application()
        .append_worker_update_request(request)
        .await
}

pub async fn create_agent_child_run(
    state: &AppState,
    request: &AgentWorkerDelegationStartRequest,
) -> Result<AgentWorkerDelegationStartSnapshotWire, AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent delegation start 无效: {error}"))
    })?;
    state
        .agent_worker_delegation_application()
        .create_child_run_request(request)
        .await
}

pub async fn agent_worker_run_is_terminal(
    state: &AppState,
    request: &AgentWorkerRunLeaseRequest,
) -> Result<AgentWorkerRunStatusSnapshotWire, AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent run status 无效: {error}")))?;
    state
        .agent_worker_completion_application()
        .run_is_terminal_request(request)
        .await
}

pub async fn authorize_agent_host_command(
    state: &AppState,
    request: &WorkerEnvelope<a4n_worker_contract::AgentHostCommandPayload>,
) -> Result<i64, AppError> {
    state.authorize_agent_host_command(request).await
}

pub async fn agent_package_execution_enabled(state: &AppState) -> Result<bool, AppError> {
    crate::actions::package::package_execution_enabled(state, PackageId::Agent).await
}
