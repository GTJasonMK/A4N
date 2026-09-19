use crate::AppState;
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
    AgentWorkerWorkflowCancelSnapshotWire, WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal,
    WorkerPrincipalKind, WorkerRequest,
};
use futures::Stream;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

mod editor;
mod history;
mod task_wait;
pub use editor::respond_agent_editor;

/// Execute Agent-native domain commands on the Control-owned Runtime.
///
/// External workers use this boundary for commands that are intentionally not
/// ordinary Web manifest routes. The domain adapters below remain the single
/// owner of resource checks and business behavior.
pub async fn dispatch_agent_host_command(
    state: &AppState,
    user_id: i64,
    scope: AgentScope,
    run_id: i64,
    turn_id: i64,
    execution_id: Option<i64>,
    lease_token: String,
    command: String,
    arguments: Value,
) -> Result<Value, AppError> {
    if !scope.allowed_tools.is_empty()
        && !scope
            .allowed_tools
            .iter()
            .any(|allowed| allowed == &command)
    {
        return Err(AppError::Forbidden(format!(
            "Agent scope 不允许使用工具: {command}"
        )));
    }
    match command.as_str() {
        "inspect_project_context" => {
            let project_id = scope
                .project_id
                .ok_or_else(|| AppError::InvalidArgument("内容优化需要绑定项目".into()))?;
            let chapter_id = scope
                .chapter_id
                .ok_or_else(|| AppError::InvalidArgument("内容优化需要绑定章节".into()))?;
            let input = parse_arguments::<
                a4n_api_contract::optimize::InspectOptimizationContextInput,
            >(arguments, &command)?;
            serde_json::to_value(
                state
                    .optimize_agent_tool_application()
                    .inspect_context(user_id, project_id, chapter_id, input)
                    .await?,
            )
            .map_err(AppError::from)
        }
        "read_project_context" => {
            let project_id = scope
                .project_id
                .ok_or_else(|| AppError::InvalidArgument("内容优化需要绑定项目".into()))?;
            let chapter_id = scope
                .chapter_id
                .ok_or_else(|| AppError::InvalidArgument("内容优化需要绑定章节".into()))?;
            let input = parse_arguments::<a4n_api_contract::optimize::ReadOptimizationContextInput>(
                arguments, &command,
            )?;
            serde_json::to_value(
                state
                    .optimize_agent_tool_application()
                    .read_context(user_id, project_id, chapter_id, input)
                    .await?,
            )
            .map_err(AppError::from)
        }
        "search_project_context" => {
            let project_id = scope
                .project_id
                .ok_or_else(|| AppError::InvalidArgument("内容优化需要绑定项目".into()))?;
            let chapter_id = scope
                .chapter_id
                .ok_or_else(|| AppError::InvalidArgument("内容优化需要绑定章节".into()))?;
            let input = parse_arguments::<
                a4n_api_contract::optimize::SearchOptimizationContextInput,
            >(arguments, &command)?;
            serde_json::to_value(
                state
                    .optimize_agent_tool_application()
                    .search_context(user_id, project_id, chapter_id, input)
                    .await?,
            )
            .map_err(AppError::from)
        }
        "edit_article" => {
            let project_id = scope
                .project_id
                .ok_or_else(|| AppError::InvalidArgument("内容优化需要绑定项目".into()))?;
            let chapter_id = scope
                .chapter_id
                .ok_or_else(|| AppError::InvalidArgument("内容优化需要绑定章节".into()))?;
            let input = parse_arguments::<
                a4n_api_contract::optimize::OptimizationArticlePatchToolInput,
            >(arguments, &command)?;
            serde_json::to_value(
                state
                    .optimize_agent_tool_application()
                    .propose_article_patch(user_id, project_id, chapter_id, turn_id, input)
                    .await?,
            )
            .map_err(AppError::from)
        }
        "apply_optimization_patches" => {
            let input = parse_arguments::<
                HostInput<a4n_api_contract::optimize::ApplyOptimizationPatchesInput>,
            >(arguments, &command)?
            .input;
            serde_json::to_value(
                crate::actions::agent_domains::content_optimization::apply_patches(
                    state, user_id, &scope, input,
                )
                .await?,
            )
            .map_err(AppError::from)
        }
        "get_generation_task_summary" | "wait_generation_task" => {
            let input = parse_arguments::<
                crate::actions::agent_domains::content_optimization::GenerationTaskSummaryInput,
            >(arguments, &command)?;
            serde_json::to_value(
                crate::actions::agent_domains::content_optimization::generation_task_summary(
                    state, user_id, &scope, input,
                )
                .await?,
            )
            .map_err(AppError::from)
        }
        "start_visual_plan_prompt_generation" => {
            let input = parse_arguments::<HostInput<StartVisualPlanPromptGenerationInput>>(
                arguments, &command,
            )?
            .input;
            serde_json::to_value(
                crate::actions::chapter_visual_plan_records::start_visual_plan_prompt_generation(
                    state, user_id, input,
                )
                .await
                .map_err(|error| AppError::InvalidArgument(error.to_string()))?,
            )
            .map_err(AppError::from)
        }
        "start_image_generation" => {
            let input =
                parse_arguments::<HostInput<StartImageGenerationInput>>(arguments, &command)?.input;
            serde_json::to_value(
                crate::actions::image_generation::start_image_generation_typed(
                    state, user_id, input,
                )
                .await
                .map_err(|error| AppError::InvalidArgument(error.to_string()))?,
            )
            .map_err(AppError::from)
        }
        "run_canvas_graph" => {
            let input =
                parse_arguments::<HostInput<RunCanvasGraphInput>>(arguments, &command)?.input;
            let origin = host_workflow_origin(run_id, execution_id, lease_token)?;
            serde_json::to_value(
                crate::actions::agent_domains::canvas::run_graph(
                    state, user_id, &scope, input, &origin,
                )
                .await?,
            )
            .map_err(AppError::from)
        }
        "run_canvas_node" => {
            let input =
                parse_arguments::<HostInput<RunCanvasNodeInput>>(arguments, &command)?.input;
            let origin = host_workflow_origin(run_id, execution_id, lease_token)?;
            serde_json::to_value(
                crate::actions::agent_domains::canvas::run_node(
                    state, user_id, &scope, input, &origin,
                )
                .await?,
            )
            .map_err(AppError::from)
        }
        "wait_workflow_run" => {
            let input = parse_arguments::<
                HostInput<a4n_api_contract::workflow::GetWorkflowRunInput>,
            >(arguments, &command)?
            .input;
            serde_json::to_value(
                crate::actions::agent_domains::canvas::get_workflow_run(
                    state, user_id, &scope, input,
                )
                .await?,
            )
            .map_err(AppError::from)
        }
        _ => Err(AppError::NotFound(format!(
            "未知 Agent synthetic Host command: {command}"
        ))),
    }
}

fn host_workflow_origin(
    run_id: i64,
    execution_id: Option<i64>,
    lease_token: String,
) -> Result<AgentWorkflowOrigin, AppError> {
    Ok(AgentWorkflowOrigin {
        run_id,
        execution_id: execution_id.ok_or_else(|| {
            AppError::InvalidArgument("Canvas Agent command 缺少 execution_id".into())
        })?,
        lease_token,
    })
}

const MAX_TOOL_PROJECT_PAGE_SIZE: u64 = 100;

pub(crate) fn worker_request_for_claimed_run(
    claimed: &AgentWorkerRun,
    expected_worker_id: &str,
    runtime_snapshot: Option<crate::runtime_config::RuntimeConfigSnapshot>,
) -> Result<WorkerRequest<AgentRunPayload>, AppError> {
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
    let request = WorkerRequest {
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

#[derive(Clone)]
pub(crate) struct RuntimeAgentToolExecutor {
    state: AppState,
    worker_request: WorkerRequest<AgentRunPayload>,
}

impl RuntimeAgentToolExecutor {
    pub(crate) fn new(state: AppState, worker_request: WorkerRequest<AgentRunPayload>) -> Self {
        Self {
            state,
            worker_request,
        }
    }

    /// 执行 Runtime 明确支持的 Agent 工具。
    ///
    /// `approved_write` 由持久控制平面提供；即使模型调用了写工具，只要没有
    /// 对应审批也会在这里显式失败。领域 action 会再次校验真实用户权限和版本。
    pub(crate) async fn execute(
        &self,
        context: &AgentRunContext,
        name: &str,
        arguments: Value,
        approved_write: bool,
        execution: Option<&AgentExecutionSnapshot>,
    ) -> Result<Value, AppError> {
        let user_id = context.owner_user_id;
        let scope = &context.scope;
        let descriptor = tool_descriptor(name)?;
        ensure_agent_scope_allows_tool(scope, name)?;
        ensure_argument_scope(scope, &descriptor, &arguments)?;
        ensure_argument_resource_scope(&self.state, user_id, scope, &arguments).await?;
        if a4n_api_contract::agent::agent_tool_requires_user_approval(name, &descriptor.effect)
            && !approved_write
        {
            return Err(AppError::Forbidden(format!(
                "Agent 工具 {name} 是写操作，必须先获得用户审批"
            )));
        }

        let result = match name {
            "locate_writing_target" => {
                let input = parse_arguments::<LocateWritingTargetInput>(arguments, name)?;
                serde_json::to_value(
                    locate_writing_target(&self.state, user_id, scope, input).await?,
                )
            }
            "list_projects" => {
                let input = parse_arguments::<ListProjectsToolInput>(arguments, name)?;
                if scope.project_id.is_some() || scope.chapter_id.is_some() {
                    return Err(AppError::Forbidden(
                        "项目/章节绑定的 Agent 不能枚举 scope 外的项目".into(),
                    ));
                }
                serde_json::to_value(
                    crate::actions::project_records::list_projects(
                        &self.state,
                        user_id,
                        input.page.unwrap_or(1),
                        input
                            .page_size
                            .unwrap_or(20)
                            .min(MAX_TOOL_PROJECT_PAGE_SIZE),
                        input.project_kind.as_deref(),
                        input.root_project_id,
                        input.access_scope.as_deref(),
                    )
                    .await
                    .map_err(AppError::InvalidArgument)?,
                )
            }
            "get_project" => {
                let input = parse_arguments::<ProjectIdToolInput>(arguments, name)?;
                ensure_project_scope(scope, input.project_id)?;
                serde_json::to_value(
                    crate::actions::project_records::get_project_summary(
                        &self.state,
                        user_id,
                        input.project_id,
                    )
                    .await
                    .map_err(AppError::InvalidArgument)?,
                )
            }
            "list_chapters" => {
                let input = parse_arguments::<ProjectIdToolInput>(arguments, name)?;
                ensure_project_scope(scope, input.project_id)?;
                serde_json::to_value(
                    crate::actions::chapter_content::list_chapters(
                        &self.state,
                        user_id,
                        input.project_id,
                    )
                    .await?,
                )
            }
            "web_search" => {
                let input = parse_arguments::<WebSearchInput>(arguments, name)?;
                serde_json::to_value(crate::actions::config_records::web_search(input).await?)
            }
            "web_fetch" => {
                let input = parse_arguments::<WebFetchInput>(arguments, name)?;
                serde_json::to_value(crate::actions::config_records::web_fetch(input).await?)
            }
            "read_image" => {
                let input = parse_arguments::<ReadImageInput>(arguments, name)?;
                serde_json::to_value(
                    crate::actions::image_records::read_image(&self.state, user_id, input).await?,
                )
            }
            "inspect_project_context" => {
                let project_id = scope
                    .project_id
                    .ok_or_else(|| AppError::InvalidArgument("内容优化需要绑定项目".into()))?;
                let chapter_id = scope
                    .chapter_id
                    .ok_or_else(|| AppError::InvalidArgument("内容优化需要绑定章节".into()))?;
                let input = parse_arguments::<
                    a4n_api_contract::optimize::InspectOptimizationContextInput,
                >(arguments, name)?;
                serde_json::to_value(
                    self.state
                        .optimize_agent_tool_application()
                        .inspect_context(user_id, project_id, chapter_id, input)
                        .await?,
                )
            }
            "read_project_context" => {
                let project_id = scope
                    .project_id
                    .ok_or_else(|| AppError::InvalidArgument("内容优化需要绑定项目".into()))?;
                let chapter_id = scope
                    .chapter_id
                    .ok_or_else(|| AppError::InvalidArgument("内容优化需要绑定章节".into()))?;
                let input = parse_arguments::<
                    a4n_api_contract::optimize::ReadOptimizationContextInput,
                >(arguments, name)?;
                serde_json::to_value(
                    self.state
                        .optimize_agent_tool_application()
                        .read_context(user_id, project_id, chapter_id, input)
                        .await?,
                )
            }
            "search_project_context" => {
                let project_id = scope
                    .project_id
                    .ok_or_else(|| AppError::InvalidArgument("内容优化需要绑定项目".into()))?;
                let chapter_id = scope
                    .chapter_id
                    .ok_or_else(|| AppError::InvalidArgument("内容优化需要绑定章节".into()))?;
                let input = parse_arguments::<
                    a4n_api_contract::optimize::SearchOptimizationContextInput,
                >(arguments, name)?;
                serde_json::to_value(
                    self.state
                        .optimize_agent_tool_application()
                        .search_context(user_id, project_id, chapter_id, input)
                        .await?,
                )
            }
            "edit_article" => {
                let project_id = scope
                    .project_id
                    .ok_or_else(|| AppError::InvalidArgument("内容优化需要绑定项目".into()))?;
                let chapter_id = scope
                    .chapter_id
                    .ok_or_else(|| AppError::InvalidArgument("内容优化需要绑定章节".into()))?;
                let input = parse_arguments::<
                    a4n_api_contract::optimize::OptimizationArticlePatchToolInput,
                >(arguments, name)?;
                serde_json::to_value(
                    self.state
                        .optimize_agent_tool_application()
                        .propose_article_patch(
                            user_id,
                            project_id,
                            chapter_id,
                            context.turn.id,
                            input,
                        )
                        .await?,
                )
            }
            "get_canvas_graph" => {
                let input =
                    parse_arguments::<HostInput<GetCanvasGraphInput>>(arguments, name)?.input;
                serde_json::to_value(
                    crate::actions::agent_domains::canvas::get_graph(
                        &self.state,
                        user_id,
                        scope,
                        input,
                    )
                    .await?,
                )
            }
            "get_canvas_node_contracts" => {
                let input = parse_arguments::<
                    HostInput<a4n_api_contract::canvas::GetCanvasNodeContractsInput>,
                >(arguments, name)?
                .input;
                serde_json::to_value(
                    crate::actions::agent_domains::canvas::get_node_contracts(
                        &self.state,
                        user_id,
                        input,
                    )
                    .await?,
                )
            }
            "save_canvas_graph" => {
                let input =
                    parse_arguments::<HostInput<SaveCanvasGraphInput>>(arguments, name)?.input;
                serde_json::to_value(
                    crate::actions::agent_domains::canvas::save_graph(
                        &self.state,
                        user_id,
                        scope,
                        input,
                    )
                    .await?,
                )
            }
            "run_canvas_graph" => {
                let input =
                    parse_arguments::<HostInput<RunCanvasGraphInput>>(arguments, name)?.input;
                serde_json::to_value(
                    crate::actions::agent_domains::canvas::run_graph(
                        &self.state,
                        user_id,
                        scope,
                        input,
                        &workflow_origin(context, execution)?,
                    )
                    .await?,
                )
            }
            "run_canvas_node" => {
                let input =
                    parse_arguments::<HostInput<RunCanvasNodeInput>>(arguments, name)?.input;
                serde_json::to_value(
                    crate::actions::agent_domains::canvas::run_node(
                        &self.state,
                        user_id,
                        scope,
                        input,
                        &workflow_origin(context, execution)?,
                    )
                    .await?,
                )
            }
            "get_workflow_run_detail" | "wait_workflow_run" => {
                let input = parse_arguments::<
                    HostInput<a4n_api_contract::workflow::GetWorkflowRunInput>,
                >(arguments, name)?
                .input;
                serde_json::to_value(
                    crate::actions::agent_domains::canvas::get_workflow_run(
                        &self.state,
                        user_id,
                        scope,
                        input,
                    )
                    .await?,
                )
            }
            "get_optimize_session" => {
                let input = parse_arguments::<
                    HostInput<a4n_api_contract::optimize::OptimizeSessionInput>,
                >(arguments, name)?
                .input;
                serde_json::to_value(
                    crate::actions::agent_domains::content_optimization::get_session(
                        &self.state,
                        user_id,
                        scope,
                        input,
                    )
                    .await?,
                )
            }
            "list_optimize_turns" => {
                let input = parse_arguments::<
                    HostInput<a4n_api_contract::optimize::OptimizeSessionInput>,
                >(arguments, name)?
                .input;
                serde_json::to_value(
                    crate::actions::agent_domains::content_optimization::list_turns(
                        &self.state,
                        user_id,
                        scope,
                        input,
                    )
                    .await?,
                )
            }
            "list_optimize_patch_sets" => {
                let input = parse_arguments::<
                    HostInput<a4n_api_contract::optimize::OptimizeSessionInput>,
                >(arguments, name)?
                .input;
                serde_json::to_value(
                    crate::actions::agent_domains::content_optimization::list_patch_sets(
                        &self.state,
                        user_id,
                        scope,
                        input,
                    )
                    .await?,
                )
            }
            "apply_optimization_patches" => {
                let input = parse_arguments::<
                    HostInput<a4n_api_contract::optimize::ApplyOptimizationPatchesInput>,
                >(arguments, name)?
                .input;
                serde_json::to_value(
                    crate::actions::agent_domains::content_optimization::apply_patches(
                        &self.state,
                        user_id,
                        scope,
                        input,
                    )
                    .await?,
                )
            }
            "get_generation_task_summary" | "wait_generation_task" => {
                let input = parse_arguments::<
                    crate::actions::agent_domains::content_optimization::GenerationTaskSummaryInput,
                >(arguments, name)?;
                serde_json::to_value(
                    crate::actions::agent_domains::content_optimization::generation_task_summary(
                        &self.state,
                        user_id,
                        scope,
                        input,
                    )
                    .await?,
                )
            }
            "save_chapter_visual_plan_draft" => {
                let mut input =
                    parse_arguments::<HostInput<SaveChapterVisualPlanDraftInput>>(arguments, name)?
                        .input;
                // Director writes are collaborative drafts by contract. Tool
                // arguments cannot turn a reviewable draft into a confirmed record.
                input.authoring_mode = ChapterVisualPlanAuthoringMode::Collaborative;
                serde_json::to_value(
                    crate::actions::chapter_visual_plan_records::save_chapter_visual_plan_draft(
                        &self.state,
                        user_id,
                        input,
                    )
                    .await
                    .map_err(AppError::InvalidArgument)?,
                )
            }
            "search_agent_tools" => {
                let mut input = parse_arguments::<SearchAgentToolsInput>(arguments, name)?;
                a4n_api_contract::agent::scope_agent_tool_search(&context.profile_id, &mut input);
                serde_json::to_value(
                    self.state
                        .agent_tool_read_application()
                        .search_tools(context.owner_user_id, input)
                        .await?,
                )
            }
            "search_agent_skills" => serde_json::to_value(
                self.state
                    .agent_skill_application()
                    .search_skills(
                        user_id,
                        &context.profile_id,
                        parse_arguments::<SearchAgentSkillsInput>(arguments, name)?,
                    )
                    .await?,
            ),
            "load_agent_skill" => serde_json::to_value(
                self.state
                    .agent_skill_application()
                    .load_skill(
                        user_id,
                        &context.profile_id,
                        parse_arguments::<LoadAgentSkillInput>(arguments, name)?,
                    )
                    .await?,
            ),
            "describe_agent_tool" => {
                let input = parse_arguments::<DescribeAgentToolInput>(arguments, name)?;
                ensure_agent_scope_allows_tool(scope, &input.name)?;
                serde_json::to_value(
                    self.state
                        .agent_tool_read_application()
                        .describe_tool(context.owner_user_id, input)
                        .await?,
                )
            }
            "read_agent_memory" => {
                let input = parse_arguments::<a4n_api_contract::agent::ReadAgentMemoryInput>(
                    arguments, name,
                )?;
                serde_json::to_value(
                    self.state
                        .agent_worker_read_application()
                        .read_memory_for_run(context, input)
                        .await?,
                )
            }
            "read_agent_execution_result" => {
                let input = parse_arguments::<
                    a4n_api_contract::agent::ReadAgentExecutionResultInput,
                >(arguments, name)?;
                serde_json::to_value(
                    self.state
                        .agent_worker_read_application()
                        .read_execution_result_for_run(context, input)
                        .await?,
                )
            }
            // `delegate` is handled by `execute_tool_call` because it creates a
            // durable child run; it must never reach the Host-command executor.
            "delegate" => unreachable!("delegate handled by Agent worker"),
            "review_agent_delegation" => {
                let input = parse_arguments::<
                    HostInput<a4n_api_contract::agent::ReviewAgentDelegationInput>,
                >(arguments, name)?
                .input;
                serde_json::to_value(
                    self.state
                        .agent_delegation_application()
                        .review_delegation(user_id, input, Some(context.run.id))
                        .await?,
                )
            }
            "inspect_editor_target" | "read_editor_draft" | "save_editor_draft" => {
                return Err(AppError::InvalidArgument(
                    "当前 Turn 没有绑定可用的原始编辑器；请提供明确资源 ID 使用已保存资源工具"
                        .into(),
                ));
            }
            _ => self
                .state
                .dispatch_agent_host_command(
                    user_id,
                    scope,
                    name.to_string(),
                    arguments,
                    &self.worker_request,
                    execution.map(|execution| execution.id),
                )
                .await
                .map_err(|error| serde_json::Error::io(std::io::Error::other(error.to_string()))),
        };
        result.map_err(AppError::Serialization)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectIdToolInput {
    project_id: i64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ListProjectsToolInput {
    page: Option<u64>,
    page_size: Option<u64>,
    project_kind: Option<String>,
    root_project_id: Option<i64>,
    access_scope: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HostInput<T> {
    input: T,
}

fn tool_descriptor(
    name: &str,
) -> Result<a4n_api_contract::command_manifest::CommandDescriptor, AppError> {
    a4n_api_contract::agent::agent_execution_tool_descriptor(name).ok_or_else(|| {
        if a4n_api_contract::command_manifest::command_descriptor(name).is_some() {
            AppError::Forbidden(format!("Host command manifest 禁止 Agent 使用工具: {name}"))
        } else {
            AppError::NotFound(format!("Host command manifest 中不存在工具: {name}"))
        }
    })
}

/// A bound Agent scope is also a resource fence for dynamically discovered
/// Host commands.  Domain adapters perform their own checks; this generic
/// pass catches project/chapter identifiers nested in manifest arguments before
/// the command dispatcher can act on another resource.
fn ensure_argument_scope(
    scope: &AgentScope,
    descriptor: &a4n_api_contract::command_manifest::CommandDescriptor,
    arguments: &Value,
) -> Result<(), AppError> {
    let mut project_ids = Vec::new();
    let mut chapter_ids = Vec::new();
    collect_scope_ids(arguments, &mut project_ids, &mut chapter_ids);
    for project_id in &project_ids {
        ensure_project_scope(scope, *project_id)?;
    }
    for chapter_id in &chapter_ids {
        ensure_chapter_scope(scope, *chapter_id)?;
    }
    // Domain adapters (Canvas/optimization) resolve resource-only identifiers
    // through their own project-aware repository checks. Generic manifest
    // commands do not have that adapter, so a project/chapter-bound Agent must
    // provide an explicit project/chapter fence and must not invoke a global
    // command such as create_project from a bound scope.
    if (scope.project_id.is_some() || scope.chapter_id.is_some() || scope.resource_kind.is_some())
        && a4n_api_contract::agent::synthetic_agent_tool_descriptor(&descriptor.name).is_none()
        && !matches!(descriptor.name.as_str(), "web_search" | "web_fetch")
        && project_ids.is_empty()
        && chapter_ids.is_empty()
    {
        return Err(AppError::Forbidden(format!(
            "Agent scope 受限时，Host command {} 必须同时提供 projectId 或 chapterId",
            descriptor.name
        )));
    }
    Ok(())
}

async fn ensure_argument_resource_scope(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    arguments: &Value,
) -> Result<(), AppError> {
    let Some(project_id) = scope.project_id else {
        return Ok(());
    };
    // A project-bound Agent may receive a chapter-only command. Verify the
    // relationship from the chapter table instead of trusting the argument
    // name or allowing a same-shaped id from another project.
    let mut chapter_ids = Vec::new();
    let mut project_ids = Vec::new();
    collect_scope_ids(arguments, &mut project_ids, &mut chapter_ids);
    if chapter_ids.is_empty() {
        return Ok(());
    }
    for requested_project in project_ids {
        ensure_project_scope(scope, requested_project)?;
    }
    state
        .agent_resource_access_application()
        .verify_chapters_in_project(user_id, project_id, &chapter_ids)
        .await
}

fn collect_scope_ids(value: &Value, project_ids: &mut Vec<i64>, chapter_ids: &mut Vec<i64>) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                let normalized = key.to_ascii_lowercase();
                if is_runtime_snapshot_key(&normalized) {
                    continue;
                }
                if is_project_scope_key(&normalized) {
                    collect_numeric_ids(value, project_ids);
                } else if is_chapter_scope_key(&normalized) {
                    collect_numeric_ids(value, chapter_ids);
                }
                collect_scope_ids(value, project_ids, chapter_ids);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_scope_ids(value, project_ids, chapter_ids);
            }
        }
        _ => {}
    }
}

fn is_project_scope_key(key: &str) -> bool {
    key == "projectid"
        || key == "project_id"
        || key == "projectids"
        || key == "project_ids"
        || key.ends_with("_project_id")
        || key.ends_with("_project_ids")
        || key.ends_with("projectid")
        || key.ends_with("projectids")
}

fn is_chapter_scope_key(key: &str) -> bool {
    key == "chapterid"
        || key == "chapter_id"
        || key == "chapterids"
        || key == "chapter_ids"
        || key.ends_with("_chapter_id")
        || key.ends_with("_chapter_ids")
        || key.ends_with("chapterid")
        || key.ends_with("chapterids")
}

fn collect_numeric_ids(value: &Value, ids: &mut Vec<i64>) {
    match value {
        Value::Number(number) => {
            if let Some(id) = number.as_i64() {
                ids.push(id);
            }
        }
        Value::String(raw) => {
            if let Ok(id) = raw.trim().parse::<i64>() {
                ids.push(id);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_numeric_ids(value, ids);
            }
        }
        _ => {}
    }
}

fn is_runtime_snapshot_key(key: &str) -> bool {
    key == "runtime_config_snapshot"
}

pub(crate) fn ensure_project_scope(scope: &AgentScope, project_id: i64) -> Result<(), AppError> {
    if project_id <= 0 {
        return Err(AppError::InvalidArgument("projectId 无效".into()));
    }
    if scope.project_id.is_some_and(|scoped| scoped != project_id) {
        return Err(AppError::Forbidden(format!(
            "Agent scope 仅允许项目 {}，不能访问项目 {project_id}",
            scope.project_id.expect("checked Some")
        )));
    }
    Ok(())
}

pub(crate) fn ensure_chapter_scope(scope: &AgentScope, chapter_id: i64) -> Result<(), AppError> {
    if chapter_id <= 0 {
        return Err(AppError::InvalidArgument("chapterId 无效".into()));
    }
    if scope.chapter_id.is_some_and(|scoped| scoped != chapter_id) {
        return Err(AppError::Forbidden(format!(
            "Agent scope 仅允许章节 {}，不能访问章节 {chapter_id}",
            scope.chapter_id.expect("checked Some")
        )));
    }
    Ok(())
}

fn parse_arguments<T: DeserializeOwned>(arguments: Value, name: &str) -> Result<T, AppError> {
    serde_json::from_value(arguments)
        .map_err(|error| AppError::InvalidArgument(format!("Agent 工具 {name} 参数无效: {error}")))
}

async fn locate_writing_target(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    input: LocateWritingTargetInput,
) -> Result<WritingTargetView, AppError> {
    let project_id = match input.project_id.or(scope.project_id) {
        Some(project_id) => {
            ensure_project_scope(scope, project_id)?;
            project_id
        }
        None => {
            crate::actions::project_records::list_projects(
                state,
                user_id,
                1,
                1,
                Some("text"),
                None,
                None,
            )
            .await
            .map_err(AppError::InvalidArgument)?
            .items
            .into_iter()
            .next()
            .ok_or_else(|| AppError::NotFound("没有可访问的写作项目".into()))?
            .id
        }
    };
    ensure_project_scope(scope, project_id)?;
    let project = crate::actions::project_records::get_project_summary(state, user_id, project_id)
        .await
        .map_err(AppError::InvalidArgument)?;
    let chapters =
        crate::actions::chapter_content::list_chapters(state, user_id, project_id).await?;
    let explicit_chapter_id = input.chapter_id.or(scope.chapter_id);
    let chapter = match explicit_chapter_id {
        Some(chapter_id) => {
            ensure_chapter_scope(scope, chapter_id)?;
            chapters
                .into_iter()
                .find(|chapter| chapter.id == chapter_id)
                .ok_or_else(|| {
                    AppError::NotFound(format!("项目 {project_id} 中不存在章节 {chapter_id}"))
                })?
        }
        None => chapters
            .into_iter()
            .filter(|chapter| input.include_empty || !chapter.content.trim().is_empty())
            .max_by_key(|chapter| {
                (
                    chapter.updated_at.clone(),
                    chapter.chapter_number,
                    chapter.id,
                )
            })
            .ok_or_else(|| AppError::NotFound("写作项目中没有符合条件的章节".into()))?,
    };
    let content_hash = a4n_domain::source_text_hash(&chapter.content);
    let selection_reason = if explicit_chapter_id.is_some() {
        "使用显式指定或 Agent scope 绑定的章节"
    } else if input.project_id.is_some() || scope.project_id.is_some() {
        "在指定项目中按更新时间、章节号选择最新章节"
    } else {
        "选择最近更新的写作项目，并按更新时间、章节号选择最新章节"
    };
    Ok(WritingTargetView {
        project: a4n_api_contract::agent::AgentResourceRef {
            kind: "project".into(),
            id: project.id.to_string(),
            label: Some(project.title),
            revision: Some(project.updated_at),
            hash: None,
        },
        chapter: a4n_api_contract::agent::AgentResourceRef {
            kind: "chapter".into(),
            id: chapter.id.to_string(),
            label: chapter.title.clone(),
            revision: Some(format!("live:{}:{content_hash}", chapter.id)),
            hash: Some(content_hash.clone()),
        },
        chapter_number: Some(chapter.chapter_number),
        content_revision: Some(format!("live:{}:{content_hash}", chapter.id)),
        content_hash: Some(content_hash),
        selection_reason: selection_reason.into(),
    })
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

pub async fn claim_agent_worker_request(
    state: &AppState,
    worker_id: &str,
) -> Result<Option<WorkerRequest<AgentRunPayload>>, AppError> {
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
    request: &WorkerRequest<AgentRunPayload>,
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
    request: &WorkerRequest<a4n_worker_contract::AgentHostCommandPayload>,
) -> Result<i64, AppError> {
    state.authorize_agent_host_command(request).await
}

pub async fn agent_package_execution_enabled(state: &AppState) -> Result<bool, AppError> {
    crate::actions::package::package_execution_enabled(state, PackageId::Agent).await
}

pub async fn list_agent_external_task_waits(
    state: &AppState,
    request: &AgentWorkerExternalTaskWaitScanRequest,
) -> Result<Vec<AgentWorkerExternalTaskWaitSnapshotWire>, AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent external task wait scan 无效: {error}"))
    })?;
    state
        .agent_external_task_wait_application()
        .external_task_waits(request.after_run_id)
        .await
        .map(|waits| {
            waits
                .into_iter()
                .map(|wait| AgentWorkerExternalTaskWaitSnapshotWire {
                    run_id: wait.run_id,
                    owner_user_id: wait.owner_user_id,
                    scope: wait.scope,
                    execution_id: wait.execution_id,
                    tool_name: wait.tool_name,
                    arguments_json: wait.arguments_json,
                    duration_exhausted: wait.duration_exhausted,
                })
                .collect()
        })
}

pub async fn list_agent_generation_cancel_candidates(
    state: &AppState,
    request: &AgentWorkerGenerationCancelScanRequest,
) -> Result<Vec<AgentWorkerGenerationCancelSnapshotWire>, AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent generation cancel scan 无效: {error}"))
    })?;
    state
        .agent_generation_cancel_application()
        .generation_cancel_candidates(&request.after_task_id)
        .await
        .map(|tasks| {
            tasks
                .into_iter()
                .map(|task| AgentWorkerGenerationCancelSnapshotWire {
                    task_id: task.task_id,
                    attempt: task.attempt,
                    requested_by_user_id: task.requested_by_user_id,
                    status: task.status,
                    owner_stopped: task.owner_stopped,
                })
                .collect()
        })
}

pub async fn list_agent_workflow_cancel_candidates(
    state: &AppState,
    request: &AgentWorkerWorkflowCancelScanRequest,
) -> Result<Vec<AgentWorkerWorkflowCancelSnapshotWire>, AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent workflow cancel scan 无效: {error}"))
    })?;
    state
        .agent_workflow_cancel_application()
        .workflow_cancel_candidates(request.after_workflow_run_id)
        .await
        .map(|workflows| {
            workflows
                .into_iter()
                .map(|workflow| AgentWorkerWorkflowCancelSnapshotWire {
                    workflow_run_id: workflow.workflow_run_id,
                    owner_stopped: workflow.owner_stopped,
                })
                .collect()
        })
}

pub async fn wake_agent_external_task_wait(
    state: &AppState,
    request: &AgentWorkerExternalTaskWaitWakeRequest,
) -> Result<(), AppError> {
    request.validate().map_err(|error| {
        AppError::InvalidArgument(format!("Agent external task wait wake 无效: {error}"))
    })?;
    state
        .agent_external_task_wait_application()
        .wake_external_task_wait(request.run_id, request.execution_id)
        .await
}

pub async fn read_external_task_wait(
    state: &AppState,
    requested: AgentExternalTaskWaitSnapshot,
    runtime_snapshot: Option<crate::runtime_config::RuntimeConfigSnapshot>,
) -> Result<Value, AppError> {
    let authoritative = state
        .agent_external_task_wait_application()
        .external_task_wait(requested.run_id, requested.execution_id)
        .await?
        .ok_or_else(|| AppError::Forbidden("Agent wait 不再属于可访问的 durable Run".into()))?;
    let owner_matches = authoritative.owner_user_id == requested.owner_user_id;
    // Restricted capabilities hydrate their default tool catalog while the
    // worker builds AgentRunContext. The durable thread scope intentionally
    // stores an empty allowlist in that case; compare the resource fence and
    // capability policy, and require an explicit stored allowlist to match.
    let scope_matches = authoritative.scope.resource_kind == requested.scope.resource_kind
        && authoritative.scope.resource_id == requested.scope.resource_id
        && authoritative.scope.project_id == requested.scope.project_id
        && authoritative.scope.chapter_id == requested.scope.chapter_id
        && authoritative.scope.allowed_capabilities == requested.scope.allowed_capabilities
        && (authoritative.scope.allowed_tools.is_empty()
            || authoritative.scope.allowed_tools == requested.scope.allowed_tools);
    let tool_matches = authoritative.tool_name == requested.tool_name;
    let arguments_match = authoritative.arguments_json == requested.arguments_json;
    if !(owner_matches && scope_matches && tool_matches && arguments_match) {
        tracing::warn!(
            run_id = requested.run_id,
            execution_id = requested.execution_id,
            owner_matches,
            scope_matches,
            tool_matches,
            arguments_match,
            "Agent wait request 与 durable execution 不匹配"
        );
        return Err(AppError::Forbidden(
            "Agent wait 请求与 durable Run/thread scope 不匹配".into(),
        ));
    }
    task_wait::read_with_snapshot(
        state,
        authoritative.owner_user_id,
        &authoritative.scope,
        &authoritative,
        runtime_snapshot.as_ref(),
    )
    .await
}

pub async fn cancel_agent_owned_generation_task(
    state: &AppState,
    user_id: i64,
    task_id: &str,
    attempt: u32,
) -> Result<(), AppError> {
    let candidate = state
        .agent_generation_cancel_application()
        .generation_cancel_candidates("")
        .await?
        .into_iter()
        .find(|task| task.task_id == task_id && task.attempt == attempt)
        .ok_or_else(|| AppError::Forbidden("生成任务不属于已停止的 Agent Run".into()))?;
    if candidate.requested_by_user_id != user_id
        || !candidate.owner_stopped
        || candidate.status == "cancelling"
    {
        return Err(AppError::Forbidden(
            "生成任务取消请求未通过 durable Agent owner 校验".into(),
        ));
    }
    crate::actions::generation_tasks::cancel_generation_task(
        state,
        candidate.requested_by_user_id,
        a4n_api_contract::generation_tasks::CancelGenerationTaskInput {
            task_id: candidate.task_id,
            attempt: candidate.attempt,
        },
    )
    .await
    .map(|_| ())
    .map_err(|error| AppError::InvalidArgument(error.to_string()))
}

pub async fn cancel_agent_owned_workflow(
    state: &AppState,
    workflow_run_id: i64,
    reason: &str,
) -> Result<(), AppError> {
    let candidate = state
        .agent_workflow_cancel_application()
        .workflow_cancel_candidates(workflow_run_id.saturating_sub(1))
        .await?
        .into_iter()
        .find(|workflow| workflow.workflow_run_id == workflow_run_id)
        .ok_or_else(|| AppError::Forbidden("Workflow 不属于 Agent durable owner".into()))?;
    if !candidate.owner_stopped {
        return Err(AppError::Forbidden(
            "只能取消已停止 Agent 所拥有的 Workflow".into(),
        ));
    }
    state
        .workflow_cancellation_application()
        .cancel(workflow_run_id, reason)
        .await
}

pub async fn agent_llm_capabilities(
    state: &AppState,
    user_id: i64,
    runtime_snapshot: Option<crate::runtime_config::RuntimeConfigSnapshot>,
    config_id: Option<i64>,
) -> Result<a4n_agent_runtime_contract::AgentLlmCapabilities, AppError> {
    crate::runtime_config::request_scope(user_id, runtime_snapshot, async {
        let provider = state.dynamic_llm_provider_for_profile()?;
        Ok(a4n_agent_runtime_contract::AgentLlmCapabilities {
            model: provider.capabilities_for_config_id(config_id).await?,
            agent_protocol: provider.agent_protocol_for_config_id(config_id).await?,
        })
    })
    .await
}

pub async fn agent_chat_stream(
    state: &AppState,
    user_id: i64,
    runtime_snapshot: Option<crate::runtime_config::RuntimeConfigSnapshot>,
    config_id: Option<i64>,
    request: ChatRequest,
) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
    crate::runtime_config::request_scope(user_id, runtime_snapshot, async {
        state
            .dynamic_llm_provider_for_profile()?
            .chat_stream_with_config_id(config_id, request)
            .await
    })
    .await
}

pub async fn prepare_agent_chat_request(
    state: &AppState,
    user_id: i64,
    runtime_snapshot: Option<crate::runtime_config::RuntimeConfigSnapshot>,
    config_id: Option<i64>,
    request: ChatRequest,
) -> Result<ChatRequest, AppError> {
    crate::runtime_config::request_scope(user_id, runtime_snapshot, async {
        state
            .dynamic_llm_provider_for_profile()?
            .prepare_chat_request_with_config_id(config_id, request)
            .await
    })
    .await
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

fn action_budget_exhausted(
    budget: &AgentBudget,
    tokens: u64,
    started_at: chrono::NaiveDateTime,
) -> bool {
    budget.max_tokens.is_some_and(|limit| tokens >= limit)
        || budget.max_duration_seconds.is_some_and(|limit| {
            chrono::Utc::now()
                .naive_utc()
                .signed_duration_since(started_at)
                .num_seconds()
                >= i64::try_from(limit).expect("Agent budget duration was validated")
        })
}

fn json_u64(value: &Value, key: &str) -> Result<u64, AppError> {
    match value.get(key) {
        None => Ok(0),
        Some(value) => value.as_u64().ok_or_else(|| AppError::InvalidState {
            current: format!("Agent usage.{key} 不是非负整数"),
            operation: "读取 Agent run".into(),
        }),
    }
}

fn should_request_approval(
    context: &AgentRunContext,
    tool_name: &str,
    effect: &str,
) -> Result<bool, AppError> {
    if effect == "read" {
        return Ok(false);
    }
    match context.execution_mode.as_str() {
        "advice_only" => Err(AppError::Forbidden(format!(
            "Agent 处于 advice_only 模式，不能执行写工具 {tool_name}"
        ))),
        "review_writes" => Ok(true),
        "autonomous" => Ok(a4n_api_contract::agent::agent_tool_requires_user_approval(
            tool_name, effect,
        )),
        mode => Err(AppError::InvalidState {
            current: format!("未知 Agent execution_mode: {mode}"),
            operation: "执行 Agent 写工具".into(),
        }),
    }
}

async fn append_tool_observation(
    message_application: &dyn a4n_agent_runtime_contract::AgentWorkerMessageApplicationPort,
    claimed: &AgentWorkerRun,
    execution: &AgentExecutionSnapshot,
    call: &ToolCall,
    value: Value,
    messages: &mut Vec<ChatMessage>,
) -> Result<(), AppError> {
    let value = history::bounded_observation(execution.id, value)?;
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

fn tool_result_refs(
    name: &str,
    value: &Value,
) -> Result<Vec<a4n_api_contract::agent::AgentResourceRef>, AppError> {
    use a4n_api_contract::agent::AgentResourceRef;
    let mut refs = value
        .get("result_refs")
        .filter(|value| !value.is_null())
        .map(|value| serde_json::from_value::<Vec<AgentResourceRef>>(value.clone()))
        .transpose()?
        .unwrap_or_default();
    if let Some(resource) = value.get("resource").filter(|value| !value.is_null()) {
        refs.push(serde_json::from_value(resource.clone())?);
    }
    if name == "save_chapter_visual_plan_draft" {
        let saved: a4n_api_contract::chapter_visual_plan::ChapterVisualPlanView =
            serde_json::from_value(value.clone())?;
        refs.push(AgentResourceRef {
            kind: "chapter_visual_plan".into(),
            id: saved.record.id.to_string(),
            revision: Some(saved.record.revision.to_string()),
            hash: Some(saved.record.plan_sha256),
            label: Some("视觉方案草稿".into()),
        });
    }
    Ok(refs)
}

fn enum_string<T: serde::Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .expect("Agent policy enum must serialize")
        .as_str()
        .expect("Agent policy enum must serialize to a string")
        .to_string()
}

pub async fn ensure_main_thread(
    state: &AppState,
    user_id: i64,
    input: EnsureMainAgentThreadInput,
) -> Result<AgentThreadView, AppError> {
    state
        .agent_thread_mutation_application()
        .ensure_main_thread(user_id, input)
        .await
}

pub async fn create_thread(
    state: &AppState,
    user_id: i64,
    input: CreateAgentThreadInput,
) -> Result<AgentThreadView, AppError> {
    state
        .agent_thread_mutation_application()
        .create_thread(user_id, input)
        .await
}

pub async fn fork_thread(
    state: &AppState,
    user_id: i64,
    input: ForkAgentThreadInput,
) -> Result<AgentThreadView, AppError> {
    state
        .agent_thread_mutation_application()
        .fork_thread(user_id, input)
        .await
}

pub async fn update_thread(
    state: &AppState,
    user_id: i64,
    input: UpdateAgentThreadInput,
) -> Result<AgentThreadView, AppError> {
    state
        .agent_thread_mutation_application()
        .update_thread(user_id, input)
        .await
}

pub async fn archive_thread(
    state: &AppState,
    user_id: i64,
    input: ArchiveAgentThreadInput,
) -> Result<AgentThreadView, AppError> {
    state
        .agent_thread_mutation_application()
        .archive_thread(user_id, input)
        .await
}

pub async fn list_threads(
    state: &AppState,
    user_id: i64,
    input: ListAgentThreadsInput,
) -> Result<Vec<AgentThreadView>, AppError> {
    state
        .agent_thread_read_application()
        .list_threads(user_id, input)
        .await
}

pub async fn show_thread(
    state: &AppState,
    user_id: i64,
    thread_id: &str,
) -> Result<AgentThreadSnapshot, AppError> {
    state
        .agent_thread_read_application()
        .show_thread(user_id, thread_id)
        .await
}

pub async fn list_messages(
    state: &AppState,
    user_id: i64,
    input: ListAgentMessagesInput,
) -> Result<Vec<a4n_api_contract::agent::AgentMessageView>, AppError> {
    state
        .agent_thread_read_application()
        .list_messages(user_id, input)
        .await
}

pub async fn show_run(
    state: &AppState,
    user_id: i64,
    run_id: &str,
) -> Result<AgentRunView, AppError> {
    state
        .agent_run_read_application()
        .show_run(user_id, run_id)
        .await
}

pub async fn send_message(
    state: &AppState,
    user_id: i64,
    input: SendAgentMessageInput,
) -> Result<AgentMessageAccepted, AppError> {
    send_message_with_admission(state, user_id, input, false).await
}

pub async fn send_message_with_admission(
    state: &AppState,
    user_id: i64,
    mut input: SendAgentMessageInput,
    require_runtime_snapshot_for_new_run: bool,
) -> Result<AgentMessageAccepted, AppError> {
    let runtime_config_snapshot = input.runtime_config_snapshot.take();
    let runtime_snapshot_required = runtime_config_snapshot.is_some();
    let _admission = state.lock_agent_admission().await;
    crate::actions::package::ensure_package_execution_enabled(state, PackageId::Agent).await?;
    let accepted = state
        .agent_run_mutation_application()
        .send_message(
            user_id,
            input,
            runtime_snapshot_required,
            require_runtime_snapshot_for_new_run,
        )
        .await?;
    if accepted.created_run
        && let Some(runtime_config_snapshot) = runtime_config_snapshot
    {
        let run_id = accepted
            .run_id
            .parse::<i64>()
            .map_err(|_| AppError::InvalidState {
                current: format!("Agent accepted run id 无效: {}", accepted.run_id),
                operation: "绑定 Agent runtime snapshot".into(),
            })?;
        state
            .store_agent_runtime_snapshot(run_id, None, runtime_config_snapshot)
            .await;
    }
    state.trigger_agent_worker();
    Ok(accepted)
}

pub async fn control_run(
    state: &AppState,
    user_id: i64,
    input: ControlAgentRunInput,
) -> Result<AgentRunView, AppError> {
    let action = input.action;
    let result = state
        .agent_run_mutation_application()
        .control_run(user_id, input)
        .await?;
    if action == a4n_api_contract::agent::AgentControlAction::Cancel {
        let run_id = result
            .id
            .parse::<i64>()
            .map_err(|_| AppError::InvalidState {
                current: format!("Agent run id 无效: {}", result.id),
                operation: "取消 Agent runtime worker".into(),
            })?;
        state.cancel_agent_run_tree(run_id).await;
        state.remove_agent_runtime_snapshot_tree(run_id).await;
        task_wait::cancel_stopped_owned_tasks(state).await?;
    }
    state.trigger_agent_worker();
    Ok(result)
}

pub async fn send_message_to_child(
    state: &AppState,
    user_id: i64,
    input: SendAgentMessageToChildInput,
) -> Result<AgentMessageAccepted, AppError> {
    send_message_to_child_with_admission(state, user_id, input, false).await
}

pub async fn send_message_to_child_with_admission(
    state: &AppState,
    user_id: i64,
    mut input: SendAgentMessageToChildInput,
    require_runtime_snapshot_for_new_run: bool,
) -> Result<AgentMessageAccepted, AppError> {
    let runtime_config_snapshot = input.runtime_config_snapshot.take();
    let runtime_snapshot_required = runtime_config_snapshot.is_some();
    let _admission = state.lock_agent_admission().await;
    crate::actions::package::ensure_package_execution_enabled(state, PackageId::Agent).await?;
    let accepted = state
        .agent_run_mutation_application()
        .send_message_to_child(
            user_id,
            input,
            runtime_snapshot_required,
            require_runtime_snapshot_for_new_run,
        )
        .await?;
    if accepted.created_run
        && let Some(runtime_config_snapshot) = runtime_config_snapshot
    {
        let run_id = accepted
            .run_id
            .parse::<i64>()
            .map_err(|_| AppError::InvalidState {
                current: format!("Agent accepted child run id 无效: {}", accepted.run_id),
                operation: "绑定 child Agent runtime snapshot".into(),
            })?;
        state
            .store_agent_runtime_snapshot(run_id, None, runtime_config_snapshot)
            .await;
    }
    state.trigger_agent_worker();
    Ok(accepted)
}

pub async fn control_child_agent(
    state: &AppState,
    user_id: i64,
    input: ControlChildAgentInput,
) -> Result<AgentRunView, AppError> {
    let action = input.action;
    let result = state
        .agent_run_mutation_application()
        .control_child_agent(user_id, input)
        .await?;
    if action == a4n_api_contract::agent::AgentControlAction::Cancel {
        let run_id = result
            .id
            .parse::<i64>()
            .map_err(|_| AppError::InvalidState {
                current: format!("Agent child run id 无效: {}", result.id),
                operation: "取消 child Agent runtime worker".into(),
            })?;
        state.cancel_agent_run_tree(run_id).await;
        state.remove_agent_runtime_snapshot_tree(run_id).await;
        task_wait::cancel_stopped_owned_tasks(state).await?;
    }
    state.trigger_agent_worker();
    Ok(result)
}

pub async fn spawn_agent(
    state: &AppState,
    user_id: i64,
    input: SpawnAgentInput,
) -> Result<a4n_api_contract::agent::AgentDelegationView, AppError> {
    spawn_agent_with_admission(state, user_id, input, false).await
}

/// Remote Hosts must bind a complete runtime snapshot before admitting a new
/// child Run. Desktop callers use the local Host path where provider
/// configuration is already resident in this process.
pub async fn spawn_agent_with_admission(
    state: &AppState,
    user_id: i64,
    mut input: SpawnAgentInput,
    require_runtime_snapshot_for_new_run: bool,
) -> Result<a4n_api_contract::agent::AgentDelegationView, AppError> {
    let parent_run_id = input
        .parent_run_id
        .parse::<i64>()
        .map_err(|_| AppError::InvalidArgument("parent_run_id 必须是正整数".into()))?;
    // A child created from a durable parent inherits the exact process-local
    // provider snapshot.  Remote callers may provide one explicitly; the
    // parent snapshot wins only when the request omitted it.
    if input.runtime_config_snapshot.is_none() {
        input.runtime_config_snapshot = state.agent_runtime_snapshot(parent_run_id).await;
    }
    let runtime_snapshot = input.runtime_config_snapshot.clone();
    if require_runtime_snapshot_for_new_run && runtime_snapshot.is_none() {
        return Err(AppError::InvalidArgument(
            "创建 Agent child run 必须提供 runtime_config_snapshot".into(),
        ));
    }
    let _admission = state.lock_agent_admission().await;
    crate::actions::package::ensure_package_execution_enabled(state, PackageId::Agent).await?;
    let result = state
        .agent_delegation_application()
        .spawn_agent(user_id, input)
        .await?;
    if let Some(runtime_snapshot) = runtime_snapshot {
        let child_run_id = result
            .child_run_id
            .as_deref()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("delegation {} 缺少 child_run_id", result.id),
                operation: "绑定 Agent child runtime snapshot".into(),
            })?
            .parse::<i64>()
            .map_err(|_| AppError::InvalidState {
                current: format!("delegation {} child_run_id 无效", result.id),
                operation: "绑定 Agent child runtime snapshot".into(),
            })?;
        state
            .store_agent_runtime_snapshot(child_run_id, Some(parent_run_id), runtime_snapshot)
            .await;
    }
    state.trigger_agent_worker();
    Ok(result)
}

pub async fn events(
    state: &AppState,
    user_id: i64,
    input: AgentEventsInput,
) -> Result<Vec<AgentEvent>, AppError> {
    state
        .agent_event_read_application()
        .events(user_id, input)
        .await
}

pub async fn search_tools(
    state: &AppState,
    user_id: i64,
    input: SearchAgentToolsInput,
) -> Result<AgentToolSearchResult, AppError> {
    state
        .agent_tool_read_application()
        .search_tools(user_id, input)
        .await
}

pub async fn describe_tool(
    state: &AppState,
    user_id: i64,
    input: DescribeAgentToolInput,
) -> Result<AgentToolDescriptor, AppError> {
    state
        .agent_tool_read_application()
        .describe_tool(user_id, input)
        .await
}

pub async fn list_skill_settings(
    state: &AppState,
    user_id: i64,
) -> Result<a4n_api_contract::agent::AgentSkillSettingsView, AppError> {
    state
        .agent_skill_application()
        .list_skill_settings(user_id)
        .await
}

pub async fn update_skill_configuration(
    state: &AppState,
    user_id: i64,
    input: a4n_api_contract::agent::UpdateAgentSkillConfigurationInput,
) -> Result<a4n_api_contract::agent::AgentSkillProfileView, AppError> {
    state
        .agent_skill_application()
        .update_skill_configuration(user_id, input)
        .await
}

pub async fn search_agent_skills_for_worker(
    state: &AppState,
    request: &AgentWorkerSkillSearchRequest,
) -> Result<AgentSkillSearchResult, AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent Skill search 无效: {error}")))?;
    state
        .agent_worker_read_application()
        .search_skills_for_worker(request)
        .await
}

pub async fn load_agent_skill_for_worker(
    state: &AppState,
    request: &AgentWorkerSkillLoadRequest,
) -> Result<AgentSkillContent, AppError> {
    request
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("Agent Skill load 无效: {error}")))?;
    state
        .agent_worker_read_application()
        .load_skill_for_worker(request)
        .await
}

pub async fn list_memories(
    state: &AppState,
    user_id: i64,
    scope_kind: Option<AgentMemoryScopeKind>,
    scope_id: Option<String>,
) -> Result<Vec<AgentMemoryView>, AppError> {
    state
        .agent_memory_read_application()
        .list_memories(user_id, scope_kind, scope_id)
        .await
}

pub async fn create_memory(
    state: &AppState,
    user_id: i64,
    input: AgentMemoryInput,
) -> Result<AgentMemoryView, AppError> {
    state
        .agent_memory_mutation_application()
        .create_memory(user_id, input)
        .await
}

pub async fn retract_memory(
    state: &AppState,
    user_id: i64,
    input: RetractAgentMemoryInput,
) -> Result<AgentMemoryView, AppError> {
    state
        .agent_memory_mutation_application()
        .retract_memory(user_id, input)
        .await
}

pub async fn review_delegation(
    state: &AppState,
    user_id: i64,
    input: a4n_api_contract::agent::ReviewAgentDelegationInput,
) -> Result<a4n_api_contract::agent::AgentDelegationView, AppError> {
    state
        .agent_delegation_application()
        .review_delegation(user_id, input, None)
        .await
}

pub async fn resolve_approval(
    state: &AppState,
    user_id: i64,
    input: ResolveAgentApprovalInput,
) -> Result<AgentApprovalView, AppError> {
    let result = state
        .agent_approval_application()
        .resolve_approval(user_id, input)
        .await?;
    state.trigger_agent_worker();
    Ok(result)
}
