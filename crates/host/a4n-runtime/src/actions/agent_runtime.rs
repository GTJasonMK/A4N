use crate::AppState;
use a4n_agent_runtime::engine::{
    action_budget_exhausted, collect_scope_ids, ensure_argument_scope, enum_string, json_u64,
    parse_arguments, should_request_approval, tool_descriptor, tool_result_refs,
};
pub(crate) use a4n_agent_runtime::engine::{ensure_chapter_scope, ensure_project_scope};
use a4n_agent_runtime_contract::{
    AgentExecutionSnapshot, AgentExternalTaskWaitSnapshot, AgentRunContext, AgentWorkerRun,
    WorkerExecutionUpdate, ensure_agent_scope_allows_tool, validate_agent_budget,
};
use a4n_api_contract::agent::{
    AgentApprovalView, AgentBudget, AgentEvent, AgentEventsInput, AgentMemoryInput,
    AgentMemoryScopeKind, AgentMemoryView, AgentMessageAccepted, AgentRunView, AgentScope,
    AgentSkillContent, AgentSkillSearchResult, AgentThreadSnapshot, AgentThreadView,
    AgentToolDescriptor, AgentToolSearchResult, ArchiveAgentThreadInput, ControlAgentRunInput,
    ControlChildAgentInput, CreateAgentThreadInput, DescribeAgentToolInput,
    EnsureMainAgentThreadInput, ForkAgentThreadInput, ListAgentMessagesInput,
    ListAgentThreadsInput, LoadAgentSkillInput, LocateWritingTargetInput,
    ResolveAgentApprovalInput, RetractAgentMemoryInput, SearchAgentSkillsInput,
    SearchAgentToolsInput, SendAgentMessageInput, SendAgentMessageToChildInput, SpawnAgentInput,
    UpdateAgentThreadInput, WritingTargetView,
};
use a4n_api_contract::canvas::{
    GetCanvasGraphInput, RunCanvasGraphInput, RunCanvasNodeInput, SaveCanvasGraphInput,
};
use a4n_api_contract::chapter_visual_plan::{
    ChapterVisualPlanAuthoringMode, SaveChapterVisualPlanDraftInput,
    StartVisualPlanPromptGenerationInput,
};
use a4n_api_contract::image_generation::StartImageGenerationInput;
use a4n_api_contract::image_read::ReadImageInput;
use a4n_api_contract::package::PackageId;
use a4n_api_contract::web_fetch::WebFetchInput;
use a4n_api_contract::web_search::WebSearchInput;
use a4n_domain::agent::AgentWorkflowOrigin;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatChunk, ChatMessage, ChatRequest, ToolCall};
use a4n_worker_contract::{
    AGENT_RUN_OPERATION, AgentRunPayload, AgentWorkerApprovalCheckRequest,
    AgentWorkerApprovalCheckSnapshotWire, AgentWorkerApprovalExpiryRequest,
    AgentWorkerApprovalRequest, AgentWorkerBudgetRequest, AgentWorkerBudgetSnapshotWire,
    AgentWorkerChapterScopeRequest, AgentWorkerCompletionRequest,
    AgentWorkerCompletionSnapshotWire, AgentWorkerContextSummaryRequest,
    AgentWorkerDelegationReviewRequest, AgentWorkerDelegationReviewSnapshotWire,
    AgentWorkerDelegationScopeRequest, AgentWorkerDelegationScopeSnapshotWire,
    AgentWorkerDelegationStartRequest, AgentWorkerDelegationStartSnapshotWire,
    AgentWorkerDelegationThreadRequest, AgentWorkerDelegationThreadSnapshotWire,
    AgentWorkerEditorWaitExpiryRequest, AgentWorkerEditorWaitRequest, AgentWorkerEventRequest,
    AgentWorkerExecutionCreateRequest, AgentWorkerExecutionLookupRequest,
    AgentWorkerExecutionResultRequest, AgentWorkerExecutionSnapshotWire,
    AgentWorkerExecutionUpdateRequest, AgentWorkerExpiredRunRecoveryRequest,
    AgentWorkerExternalTaskWaitScanRequest, AgentWorkerExternalTaskWaitSnapshotWire,
    AgentWorkerExternalTaskWaitWakeRequest, AgentWorkerFailureRequest,
    AgentWorkerGenerationCancelScanRequest, AgentWorkerGenerationCancelSnapshotWire,
    AgentWorkerHistoryRequest, AgentWorkerHistorySnapshotWire, AgentWorkerInboxClaimRequest,
    AgentWorkerInboxConsumeRequest, AgentWorkerInboxMessageRequest, AgentWorkerInboxPersistRequest,
    AgentWorkerInboxPersistSnapshotWire, AgentWorkerInboxSnapshotWire, AgentWorkerMemoryRequest,
    AgentWorkerMessageRequest, AgentWorkerMessageSnapshotWire, AgentWorkerRecoveryRequest,
    AgentWorkerResultRefsRequest, AgentWorkerResultRefsSnapshotWire, AgentWorkerRunContextWire,
    AgentWorkerRunLeaseRequest, AgentWorkerRunStatusSnapshotWire, AgentWorkerRunWaitRequest,
    AgentWorkerSkillLoadRequest, AgentWorkerSkillSearchRequest, AgentWorkerToolDescribeRequest,
    AgentWorkerToolSearchRequest, AgentWorkerUsageRequest, AgentWorkerWorkflowCancelScanRequest,
    AgentWorkerWorkflowCancelSnapshotWire, WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerLease,
    WorkerPrincipal, WorkerPrincipalKind,
};
use futures::Stream;
use serde::Deserialize;
use serde_json::{Value, json};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

mod commands;
mod editor;
mod history;
mod optimization;
mod task_wait;

pub use commands::mutations::*;
pub use commands::reads::*;
pub use editor::respond_agent_editor;
pub use history::*;
pub(crate) use optimization::RuntimeAgentToolExecutor;
pub use optimization::dispatch_agent_host_command;

pub(crate) fn worker_request_for_claimed_run(
    claimed: &AgentWorkerRun,
    expected_worker_id: &str,
    runtime_snapshot: Option<crate::runtime_config::RuntimeConfigSnapshot>,
) -> Result<WorkerEnvelope<AgentRunPayload>, AppError> {
    let run = &claimed.run;
    let worker_id = run
        .worker_id
        .as_deref()
        .ok_or_else(|| AppError::InvalidState {
            current: format!("Agent run {} 领取后缺少 worker_id", run.id),
            operation: "构造 Agent worker request".into(),
        })?;
    if worker_id != expected_worker_id {
        return Err(AppError::InvalidState {
            current: format!("Agent run {} 的 worker_id 与当前 worker 不一致", run.id),
            operation: "构造 Agent worker request".into(),
        });
    }
    if run.lease_token.as_deref() != Some(claimed.lease_token.as_str()) {
        return Err(AppError::InvalidState {
            current: format!("Agent run {} 的 lease fencing token 不一致", run.id),
            operation: "构造 Agent worker request".into(),
        });
    }
    let attempt = u64::try_from(run.attempt).map_err(|_| AppError::InvalidState {
        current: format!("Agent run {} 的 attempt 无效", run.id),
        operation: "构造 Agent worker request".into(),
    })?;
    let expires_at = run
        .lease_expires_at
        .ok_or_else(|| AppError::InvalidState {
            current: format!("Agent run {} 领取后缺少 lease_expires_at", run.id),
            operation: "构造 Agent worker request".into(),
        })?
        .to_string();
    let target_revision =
        u64::try_from(claimed.thread.revision).map_err(|_| AppError::InvalidState {
            current: format!("Agent thread {} 的 revision 无效", claimed.thread.id),
            operation: "构造 Agent worker request".into(),
        })?;
    let runtime_config_snapshot = runtime_snapshot.map(serde_json::to_value).transpose()?;
    let request = WorkerEnvelope {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: format!("agent-run-{}-{}", run.id, attempt),
        trace_id: format!("agent-run:{}:{}", run.id, attempt),
        principal: WorkerPrincipal {
            kind: WorkerPrincipalKind::User,
            subject: claimed.thread.owner_user_id.to_string(),
        },
        operation_id: AGENT_RUN_OPERATION.into(),
        idempotency_key: format!("agent-run:{}:{}", run.id, attempt),
        project_id: claimed.thread.project_id,
        root_project_id: None,
        target_revision: Some(target_revision),
        runtime_config_snapshot_ref: None,
        lease: WorkerLease {
            worker_id: worker_id.into(),
            epoch: attempt,
            fence_token: claimed.lease_token.clone(),
            expires_at,
        },
        payload: AgentRunPayload {
            run_id: run.id,
            thread_id: run.thread_id,
            turn_id: run.turn_id,
            profile_id: claimed.thread.profile_id.clone(),
            worker_run: claimed.to_wire(),
            runtime_config_snapshot,
        },
    };
    request
        .validate_agent_run()
        .map_err(|error| AppError::InvalidState {
            current: format!("Agent worker request contract 无效: {error}"),
            operation: "校验 Agent worker request".into(),
        })?;
    Ok(request)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HostInput<T> {
    input: T,
}

/// Run the shared Agent engine against the native Runtime Host.
pub(crate) async fn run_worker(state: AppState, cancel: CancellationToken, notify: Arc<Notify>) {
    let shared = state.agent_runtime_shared();
    let host = Arc::new(crate::agent_runtime_host::NativeAgentRuntimeHost::new(
        state,
    ));
    let runtime = a4n_agent_runtime::AgentRuntimeState::with_shared(host, shared);
    a4n_agent_runtime::engine::run_worker(runtime, cancel, notify).await;
}

pub fn project_agent_editor_draft(
    state: &AppState,
    plan: &a4n_api_contract::chapter_visual_plan::ChapterVisualPlanV3,
    view: &a4n_api_contract::chapter_visual_plan::VisualPlanReadView,
) -> Result<a4n_api_contract::chapter_visual_plan::VisualPlanProjection, AppError> {
    state
        .chapter_visual_plan_read_application()
        .project_editor(plan, view)
}

fn workflow_origin(
    context: &AgentRunContext,
    execution: Option<&AgentExecutionSnapshot>,
) -> Result<AgentWorkflowOrigin, AppError> {
    Ok(AgentWorkflowOrigin {
        run_id: context.run.id,
        execution_id: execution
            .ok_or_else(|| AppError::InvalidArgument("Canvas 启动缺少 Agent Execution".into()))?
            .id,
        lease_token: context
            .run
            .lease_token
            .clone()
            .ok_or_else(|| AppError::InvalidArgument("Canvas 启动缺少 Agent lease".into()))?,
    })
}

async fn append_tool_observation(
    message_application: &dyn a4n_agent_runtime_contract::AgentWorkerMessageApplicationPort,
    claimed: &AgentWorkerRun,
    execution: &AgentExecutionSnapshot,
    call: &ToolCall,
    value: Value,
    messages: &mut Vec<ChatMessage>,
) -> Result<(), AppError> {
    let value = a4n_agent_runtime::engine::bounded_observation(execution.id, value)?;
    message_application
        .append_tool_message(
            claimed,
            execution.id,
            &call.name,
            Some(&call.id),
            value.clone(),
        )
        .await?;
    let mut message = ChatMessage::new(
        a4n_llm_provider_contract::provider::MessageRole::Tool,
        serde_json::to_string(&value)?,
    );
    message.tool_call_id = Some(call.id.clone());
    messages.push(message);
    Ok(())
}
