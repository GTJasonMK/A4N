use crate::AppState;
use a4n_api_contract::agent::{
    AgentApprovalView, AgentBudget, AgentEvent, AgentEventsInput, AgentMemoryInput,
    AgentMemoryScopeKind, AgentMemoryView, AgentMessageAccepted, AgentRunView, AgentScope,
    AgentThreadSnapshot, AgentThreadView, AgentToolDescriptor, AgentToolExposure,
    AgentToolSearchResult, ArchiveAgentThreadInput, ControlAgentRunInput, ControlChildAgentInput,
    CreateAgentThreadInput, DescribeAgentToolInput, EnsureMainAgentThreadInput,
    ForkAgentThreadInput, ListAgentMessagesInput, ListAgentThreadsInput, LocateWritingTargetInput,
    ResolveAgentApprovalInput, RetractAgentMemoryInput, SearchAgentToolsInput,
    SendAgentMessageInput, SendAgentMessageToChildInput, SpawnAgentInput, UpdateAgentThreadInput,
    WritingTargetView,
};
use a4n_api_contract::canvas::{
    GetCanvasGraphInput, RunCanvasGraphInput, RunCanvasNodeInput, SaveCanvasGraphInput,
};
use a4n_api_contract::chapter_visual_plan::{
    ChapterVisualPlanAuthoringMode, SaveChapterVisualPlanDraftInput,
};
use a4n_core::error::AppError;
use a4n_llm::provider::{
    ChatMessage, ChatRequest, ChatResponse, LlmTraceContext, MessageRole, ToolCall,
};
use a4n_orchestrator::agent::{
    AgentRunContext, AgentWorkerRun, WorkerDelegationInput, WorkerExecutionInput,
    WorkerExecutionUpdate, WorkerUsage, child_budget_within_parent, enforce_delegation_limits,
    validate_agent_budget,
};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{Duration, MissedTickBehavior};
use tokio_util::sync::CancellationToken;

/// Agent 宿主 worker 的轮询间隔。
///
/// Run 的真正领取/执行由 `AgentApplication` 持久编排边界负责；本模块只持有
/// worker 生命周期和唤醒入口，避免在 Runtime 再复制一份 Agent 状态机。
const WORKER_POLL_INTERVAL: Duration = Duration::from_secs(2);
const WORKER_RENEW_INTERVAL: Duration = Duration::from_secs(10);
const WORKER_MAX_CLAIMS_PER_WAKEUP: usize = 8;
const WORKER_MAX_STEPS: u32 = 32;
const WORKER_MAX_TOOLS: u32 = 64;
const WORKER_MAX_OUTPUT_TOKENS: usize = 8_192;
const WORKER_APPROVAL_TTL_SECONDS: i64 = 15 * 60;
const MAX_TOOL_PROJECT_PAGE_SIZE: u64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunProgress {
    Continue,
    Released,
}

#[derive(Clone)]
pub(crate) struct RuntimeAgentToolExecutor {
    state: AppState,
}

impl RuntimeAgentToolExecutor {
    pub(crate) fn new(state: AppState) -> Self {
        Self { state }
    }

    /// 执行 Runtime 明确支持的 Agent 工具。
    ///
    /// `approved_write` 由持久控制平面提供；即使模型调用了写工具，只要没有
    /// 对应审批也会在这里显式失败。领域 action 会再次校验真实用户权限和版本。
    pub(crate) async fn execute(
        &self,
        user_id: i64,
        scope: &AgentScope,
        name: &str,
        arguments: Value,
        approved_write: bool,
    ) -> Result<Value, AppError> {
        let name = normalized_tool_name(name);
        let descriptor = tool_descriptor(name)?;
        ensure_scope_allows_tool(scope, name)?;
        ensure_argument_scope(scope, &descriptor, &arguments)?;
        ensure_argument_resource_scope(&self.state, user_id, scope, &arguments).await?;
        if tool_requires_approval(name, &descriptor.effect) && !approved_write {
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
                    )
                    .await?,
                )
            }
            "get_workflow_run_detail" => {
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
            "ensure_content_optimization_session" => {
                let input = parse_arguments::<crate::actions::agent_domains::content_optimization::EnsureContentOptimizationSessionInput>(arguments, name)?;
                serde_json::to_value(
                    crate::actions::agent_domains::content_optimization::ensure_session(
                        &self.state,
                        user_id,
                        scope,
                        input,
                    )
                    .await?,
                )
            }
            "send_content_optimization_message" => {
                let input = parse_arguments::<crate::actions::agent_domains::content_optimization::SendContentOptimizationMessageInput>(arguments, name)?;
                serde_json::to_value(
                    crate::actions::agent_domains::content_optimization::send_message(
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
            "get_generation_task_summary" => {
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
                let input = parse_arguments::<SearchAgentToolsInput>(arguments, name)?;
                serde_json::to_value(self.state.orchestrator().agent().search_tools(input)?)
            }
            "describe_agent_tool" => {
                let input = parse_arguments::<DescribeAgentToolInput>(arguments, name)?;
                serde_json::to_value(self.state.orchestrator().agent().describe_tool(input)?)
            }
            // `delegate` is handled by `execute_tool_call` because it creates a
            // durable child run; it must never reach the Host-command executor.
            "delegate" => unreachable!("delegate handled by Agent worker"),
            _ => self
                .state
                .dispatch_agent_host_command(user_id, name.to_string(), arguments)
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

fn normalized_tool_name(name: &str) -> &str {
    name.trim()
}

fn tool_descriptor(
    name: &str,
) -> Result<a4n_api_contract::command_manifest::CommandDescriptor, AppError> {
    if let Some(descriptor) = a4n_api_contract::agent::synthetic_agent_tool_descriptor(name) {
        return Ok(descriptor);
    }
    let descriptor = a4n_api_contract::command_manifest::command_descriptor(name)
        .ok_or_else(|| AppError::NotFound(format!("Host command manifest 中不存在工具: {name}")))?;
    let manifest_allows = descriptor
        .agent_policy
        .as_ref()
        .is_some_and(|policy| policy.exposure != AgentToolExposure::Hidden);
    // Domain adapters may intentionally expose a Host operation that carries a
    // full runtime snapshot (for example Canvas execution) and therefore has
    // no generic manifest policy. Keep those explicitly registered adapters
    // narrow; every other command must be manifest-discoverable.
    if !manifest_allows {
        return Err(AppError::Forbidden(format!(
            "Host command manifest 禁止 Agent 使用工具: {name}"
        )));
    }
    Ok(descriptor)
}

fn ensure_scope_allows_tool(scope: &AgentScope, name: &str) -> Result<(), AppError> {
    if scope.allowed_tools.is_empty() || scope.allowed_tools.iter().any(|allowed| allowed == name) {
        Ok(())
    } else {
        Err(AppError::Forbidden(format!(
            "Agent scope 不允许使用工具: {name}"
        )))
    }
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
        .orchestrator()
        .agent()
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

fn tool_requires_approval(tool_name: &str, effect: &str) -> bool {
    effect == "destructive" || crate::actions::agent_domains::requires_user_approval(tool_name)
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
    let content_hash = a4n_core::source_spans::source_text_hash(&chapter.content);
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

/// 运行 Agent worker 生命周期。
///
/// Worker 持有 durable run driver 的生命周期；领取、恢复和状态机转移仍由
/// `AgentApplication` 负责，确保 Runtime 不复制控制平面规则。
pub(crate) async fn run_worker(state: AppState, cancel: CancellationToken, notify: Arc<Notify>) {
    let mut interval = tokio::time::interval(WORKER_POLL_INTERVAL);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    // interval 的第一次 tick 立即就绪，启动时主动扫描一次，避免错过启动前已排队的 Run。
    interval.tick().await;
    drain_runnable_runs(&state).await;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = notify.notified() => {
                drain_runnable_runs(&state).await;
            }
            _ = interval.tick() => {
                drain_runnable_runs(&state).await;
            }
        }
    }
}

async fn drain_runnable_runs(state: &AppState) {
    let agent = state.orchestrator().agent();
    let worker_id = format!("a4n-agent:{}", std::process::id());
    if let Err(error) = agent.expire_pending_approvals(32).await {
        tracing::warn!(error = %error, "Agent approval expiry scan failed");
    }
    if let Err(error) = agent
        .recover_expired_runs(16, Some("Agent worker lease expired"))
        .await
    {
        tracing::warn!(error = %error, "Agent expired-run recovery failed");
    }

    for _ in 0..WORKER_MAX_CLAIMS_PER_WAKEUP {
        let admission = state.lock_agent_admission().await;
        let claimed = match agent.claim_worker_run(&worker_id).await {
            Ok(Some(run)) => run,
            Ok(None) => {
                drop(admission);
                break;
            }
            Err(error) => {
                drop(admission);
                tracing::error!(error = %error, "Agent worker failed to claim run");
                break;
            }
        };
        drop(admission);
        let run_id = claimed.run.id;
        if let Err(error) = process_claimed_run(state, claimed).await {
            tracing::error!(run_id, error = %error, "Agent worker run failed");
        }
    }
}

async fn process_claimed_run(state: &AppState, claimed: AgentWorkerRun) -> Result<(), AppError> {
    let agent = state.orchestrator().agent().clone();
    let run_cancel = state
        .register_agent_run_cancellation(claimed.run.id, claimed.run.parent_run_id)
        .await;
    let renew_cancel = CancellationToken::new();
    let renew_token = renew_cancel.clone();
    let lost_lease = run_cancel.clone();
    let renew_claim = claimed.clone();
    let renew_agent = agent.clone();
    let renew_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(WORKER_RENEW_INTERVAL);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = renew_token.cancelled() => break,
                _ = interval.tick() => {
                    if let Err(error) = renew_agent.renew_worker_lease(&renew_claim).await {
                        tracing::warn!(run_id = renew_claim.run.id, error = %error, "Agent worker lease renewal failed");
                        lost_lease.cancel();
                        break;
                    }
                }
            }
        }
    });

    let result = process_claimed_run_inner(state, &claimed, &run_cancel).await;
    renew_cancel.cancel();
    renew_handle.abort();
    state
        .unregister_agent_run_cancellation(claimed.run.id)
        .await;
    let mut terminal = false;
    if let Err(error) = &result
        && !run_cancel.is_cancelled()
    {
        match agent.fail_child_run(&claimed, error.to_string()).await {
            Ok(_) => terminal = true,
            Err(failure) => {
                tracing::warn!(run_id = claimed.run.id, error = %failure, "Agent worker could not persist failure");
            }
        }
    }
    if result.is_ok()
        && let Ok(run) = agent
            .show_run(claimed.thread.owner_user_id, &claimed.run.id.to_string())
            .await
    {
        terminal = run.status.is_terminal();
    }
    if terminal {
        let _ = state.take_agent_runtime_snapshot(claimed.run.id).await;
    }
    result
}

async fn process_claimed_run_inner(
    state: &AppState,
    claimed: &AgentWorkerRun,
    cancel: &CancellationToken,
) -> Result<(), AppError> {
    let agent = state.orchestrator().agent();
    let director_loading =
        claimed.thread.profile_id == a4n_api_contract::agent::DIRECTOR_AGENT_CAPABILITY;
    if director_loading {
        agent
            .append_worker_update(
                claimed,
                "director_context_loading",
                "published",
                "正在加载导演编排上下文",
                Some(director_progress_payload(claimed, "context", 1, None)),
            )
            .await?;
    }
    let context = agent.load_run_context(claimed).await?;
    let snapshot = state.agent_runtime_snapshot(claimed.run.id).await;
    if context.runtime_snapshot_required && snapshot.is_none() {
        agent
            .mark_recovery_required(
                claimed,
                "Agent runtime config snapshot 已丢失，无法安全恢复模型或工具调用",
            )
            .await?;
        return Ok(());
    }
    agent.mark_turn_active(claimed).await?;
    let mut messages = chat_messages(&context)?;
    let mut loaded_message_ids = context.messages.iter().map(|message| message.id).collect();
    append_inbox_messages(agent, claimed, &mut messages, &mut loaded_message_ids).await?;
    if messages.is_empty() {
        return Err(AppError::InvalidState {
            current: format!("Agent run {} 没有可发送的消息", claimed.run.id),
            operation: "执行 Agent run".into(),
        });
    }
    if director_loading {
        agent
            .append_worker_update(
                claimed,
                "director_context_ready",
                "published",
                "导演编排上下文已就绪",
                Some(director_progress_payload(claimed, "context", 1, None)),
            )
            .await?;
    }

    let budget: AgentBudget =
        serde_json::from_value(context.budget.clone()).map_err(|error| AppError::InvalidState {
            current: format!("Agent run {} budget_json 无效: {error}", claimed.run.id),
            operation: "执行 Agent run".into(),
        })?;
    validate_agent_budget(&budget)?;
    let max_tool_calls = budget.max_tool_calls.unwrap_or(WORKER_MAX_TOOLS);
    let mut usage = usage_object(&context.usage)?;
    let mut steps = json_u32(&usage, "steps_used")?;
    let mut model_calls = json_u32(&usage, "model_calls_used")?;
    let mut tool_calls = json_u32(&usage, "tool_calls_used")?;
    let mut tokens = json_u64(&usage, "tokens_used")?;
    let started_at = context.run.started_at.unwrap_or(context.run.created_at);
    let config_id = context
        .turn
        .model_override_json
        .as_ref()
        .and_then(|value| value.get("model_config_id"))
        .and_then(Value::as_i64)
        .or(context.default_model_config_id);
    // Keep discovered tools in the worker only. They are re-established from
    // the durable search/describe observation after a restart, while the
    // manifest remains the single source of executable schemas and policy.
    let mut discovered_tools = Vec::new();
    let executor = RuntimeAgentToolExecutor::new(state.clone());

    loop {
        if cancel.is_cancelled() {
            return Ok(());
        }
        append_inbox_messages(agent, claimed, &mut messages, &mut loaded_message_ids).await?;
        if steps >= budget.max_steps.unwrap_or(WORKER_MAX_STEPS)
            || model_calls >= budget.max_model_calls.unwrap_or(WORKER_MAX_STEPS)
            || budget.max_tokens.is_some_and(|limit| tokens >= limit)
            || budget.max_duration_seconds.is_some_and(|limit| {
                chrono::Utc::now()
                    .naive_utc()
                    .signed_duration_since(started_at)
                    .num_seconds()
                    >= i64::try_from(limit).expect("Agent budget duration was validated")
            })
        {
            persist_usage(
                agent,
                claimed,
                &usage,
                steps,
                model_calls,
                tool_calls,
                tokens,
            )
            .await?;
            agent.wait_run(claimed, "budget").await?;
            return Ok(());
        }
        steps = steps.saturating_add(1);
        model_calls = model_calls.saturating_add(1);
        usage["steps_used"] = json!(steps);
        usage["model_calls_used"] = json!(model_calls);
        if director_loading {
            agent
                .append_worker_update(
                    claimed,
                    "director_plan_drafting",
                    "published",
                    "导演 Agent 正在编排视觉方案",
                    Some(director_progress_payload(claimed, "draft", 2, None)),
                )
                .await?;
        }
        let tools =
            available_runtime_tools(&context.profile_id, &context.scope, &discovered_tools)?;
        let request = ChatRequest {
            messages: with_agent_system_prompt(messages.clone(), &context),
            temperature: Some(0.2),
            max_tokens: Some(WORKER_MAX_OUTPUT_TOKENS),
            max_tokens_profile: None,
            response_format: None,
            stream_include_usage: true,
            cache_scope_key: Some(format!("agent-thread:{}", context.thread.id)),
            cache_system_prefix: true,
            cache_user_prefix_len: None,
            trace_context: Some(LlmTraceContext {
                project_id: context.scope.project_id,
                root_project_id: None,
                workflow: "agent".into(),
                step_key: format!("run:{}:model:{model_calls}", claimed.run.id),
                generation_run_id: None,
                workflow_run_id: None,
                call_key: Some(format!("model-call:{model_calls}")),
                input_fingerprint_sha256: None,
                context_plan_json: None,
                context_pack_refs_json: None,
            }),
            context_manifest_json: None,
            tools: (!tools.is_empty()).then_some(tools.clone()),
            tool_choice: None,
            stop_sequences: None,
        };
        let model_execution = agent
            .create_prepared_execution(WorkerExecutionInput {
                run_id: claimed.run.id,
                lease_token: claimed.lease_token.clone(),
                execution_key: format!("model-call:{model_calls}"),
                execution_kind: "model_call".into(),
                tool_name: None,
                effect: Some("read".into()),
                // Model requests have no user-visible side effect; use the
                // canonical replay policy wire value so the durable execution
                // can be projected and resumed after a worker restart.
                replay_policy: Some("safe".into()),
                arguments_json: Some(serde_json::to_value(&request)?),
                resource_preconditions_json: None,
            })
            .await?;
        let replayed_model_execution = model_execution.status == "succeeded";
        let response: ChatResponse = if replayed_model_execution {
            serde_json::from_value(model_execution.result_json.clone().ok_or_else(|| {
                AppError::InvalidState {
                    current: format!(
                        "Agent model execution {} succeeded 但缺少 result_json",
                        model_execution.id
                    ),
                    operation: "恢复 Agent model execution".into(),
                }
            })?)?
        } else {
            agent
                .update_worker_execution(WorkerExecutionUpdate {
                    run_id: claimed.run.id,
                    lease_token: claimed.lease_token.clone(),
                    execution_id: model_execution.id,
                    status: "running".into(),
                    result_json: None,
                    result_summary: None,
                    result_refs_json: None,
                    error_message: None,
                })
                .await?;
            let provider = state.dynamic_llm_provider_for_profile()?;
            let call = provider.chat_with_config_id(config_id, request);
            let snapshot = state.agent_runtime_snapshot(claimed.run.id).await;
            let call = async {
                match snapshot {
                    Some(snapshot) => {
                        crate::runtime_config::request_scope(
                            context.owner_user_id,
                            Some(snapshot),
                            call,
                        )
                        .await
                    }
                    None => call.await,
                }
            };
            let response = tokio::select! {
                _ = cancel.cancelled() => return Ok(()),
                response = call => response?,
            };
            agent
                .update_worker_execution(WorkerExecutionUpdate {
                    run_id: claimed.run.id,
                    lease_token: claimed.lease_token.clone(),
                    execution_id: model_execution.id,
                    status: "succeeded".into(),
                    result_json: Some(serde_json::to_value(&response)?),
                    result_summary: Some("Agent model call completed".into()),
                    result_refs_json: None,
                    error_message: None,
                })
                .await?;
            response
        };
        // A succeeded durable execution is a replay after a worker restart;
        // its usage was already persisted before the crash. Counting it
        // again would let retries silently exceed token budgets.
        if !replayed_model_execution {
            if let Some(info) = response.usage.as_ref() {
                tokens = tokens.saturating_add(info.total_tokens as u64);
            } else if budget.max_tokens.is_some() {
                return Err(AppError::InvalidState {
                    current: format!(
                        "Agent run {} 的模型响应缺少 usage，无法执行 max_tokens 预算",
                        claimed.run.id
                    ),
                    operation: "执行 Agent model call".into(),
                });
            }
        }
        usage["tokens_used"] = json!(tokens);
        persist_usage(
            agent,
            claimed,
            &usage,
            steps,
            model_calls,
            tool_calls,
            tokens,
        )
        .await?;
        if budget.max_tokens.is_some_and(|limit| tokens >= limit)
            || budget.max_duration_seconds.is_some_and(|limit| {
                chrono::Utc::now()
                    .naive_utc()
                    .signed_duration_since(started_at)
                    .num_seconds()
                    >= i64::try_from(limit).expect("Agent budget duration was validated")
            })
        {
            agent.wait_run(claimed, "budget").await?;
            return Ok(());
        }
        let calls = response.message.tool_calls.clone().unwrap_or_default();
        let assistant_content = response.message.content.clone();
        if calls.is_empty() {
            if assistant_content.trim().is_empty() {
                return Err(AppError::LlmError(
                    "Agent model returned empty final content".into(),
                ));
            }
            usage["tokens_used"] = json!(tokens);
            usage["tool_calls_used"] = json!(tool_calls);
            persist_usage(
                agent,
                claimed,
                &usage,
                steps,
                model_calls,
                tool_calls,
                tokens,
            )
            .await?;
            if claimed.run.parent_run_id.is_some() {
                agent
                    .complete_child_run(claimed, assistant_content.clone(), assistant_content, None)
                    .await?;
            } else {
                agent
                    .complete_claimed_run(claimed, assistant_content, None)
                    .await?;
            }
            let _ = state.take_agent_runtime_snapshot(claimed.run.id).await;
            return Ok(());
        }
        let mut assistant = ChatMessage::assistant(assistant_content);
        assistant.tool_calls = Some(calls.clone());
        // Persist the assistant decision before dispatching side effects so a
        // resumed run can reconstruct the exact tool-call context.
        agent
            .append_assistant_tool_message(
                claimed,
                response.message.content.clone(),
                serde_json::to_value(&calls)?,
            )
            .await?;
        messages.push(assistant);
        if execute_tool_calls(
            state,
            agent,
            claimed,
            &context,
            &executor,
            calls,
            budget.max_parallel_tools,
            max_tool_calls,
            &mut discovered_tools,
            &mut messages,
            &mut usage,
            &mut tool_calls,
            steps,
            model_calls,
            tokens,
        )
        .await?
            == RunProgress::Released
        {
            return Ok(());
        }
        usage["tool_calls_used"] = json!(tool_calls);
        usage["tokens_used"] = json!(tokens);
        agent
            .update_run_usage(WorkerUsage {
                run_id: claimed.run.id,
                lease_token: claimed.lease_token.clone(),
                usage_json: usage.clone(),
            })
            .await?;
    }
}

/// Executes independent read-only calls with bounded concurrency while keeping
/// write/delegation calls ordered. Every call still goes through the durable
/// execution record and the same lease/approval checks.
async fn execute_tool_calls(
    state: &AppState,
    agent: &a4n_orchestrator::agent::AgentApplication,
    claimed: &AgentWorkerRun,
    context: &AgentRunContext,
    executor: &RuntimeAgentToolExecutor,
    calls: Vec<ToolCall>,
    max_parallel_tools: Option<u16>,
    max_tool_calls: u32,
    discovered_tools: &mut Vec<String>,
    messages: &mut Vec<ChatMessage>,
    usage: &mut Value,
    tool_calls: &mut u32,
    steps: u32,
    model_calls: u32,
    tokens: u64,
) -> Result<RunProgress, AppError> {
    let parallelism = usize::from(max_parallel_tools.unwrap_or(1).max(1));
    let mut index = 0;
    while index < calls.len() {
        let is_read = tool_descriptor(&calls[index].name)?.effect == "read";
        if !is_read || parallelism == 1 {
            let call = calls[index].clone();
            index += 1;
            *tool_calls = (*tool_calls).saturating_add(1);
            if *tool_calls > max_tool_calls {
                persist_usage(
                    agent,
                    claimed,
                    usage,
                    steps,
                    model_calls,
                    *tool_calls,
                    tokens,
                )
                .await?;
                agent.wait_run(claimed, "budget").await?;
                return Ok(RunProgress::Released);
            }
            let (progress, names) =
                execute_tool_call(state, agent, claimed, context, executor, call, messages).await?;
            extend_discovered_tools(discovered_tools, names, &context.scope)?;
            if progress == RunProgress::Released {
                return Ok(RunProgress::Released);
            }
            continue;
        }

        let end = calls[index..]
            .iter()
            .take(parallelism)
            .take_while(|call| {
                tool_descriptor(&call.name)
                    .map(|descriptor| descriptor.effect == "read")
                    .unwrap_or(false)
            })
            .count()
            .max(1)
            + index;
        let batch = calls[index..end].to_vec();
        index = end;
        let mut futures = Vec::with_capacity(batch.len());
        for call in batch {
            *tool_calls = (*tool_calls).saturating_add(1);
            if *tool_calls > max_tool_calls {
                persist_usage(
                    agent,
                    claimed,
                    usage,
                    steps,
                    model_calls,
                    *tool_calls,
                    tokens,
                )
                .await?;
                agent.wait_run(claimed, "budget").await?;
                return Ok(RunProgress::Released);
            }
            futures.push(async move {
                let mut local_messages = Vec::new();
                let (progress, names) = execute_tool_call(
                    state,
                    agent,
                    claimed,
                    context,
                    executor,
                    call,
                    &mut local_messages,
                )
                .await?;
                Ok::<_, AppError>((progress, names, local_messages))
            });
        }
        for result in futures::future::join_all(futures).await {
            let (progress, names, local_messages) = result?;
            extend_discovered_tools(discovered_tools, names, &context.scope)?;
            messages.extend(local_messages);
            if progress == RunProgress::Released {
                return Ok(RunProgress::Released);
            }
        }
    }
    Ok(RunProgress::Continue)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DelegateToolInput {
    capability: String,
    task: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    project_id: Option<i64>,
    #[serde(default)]
    chapter_id: Option<i64>,
    #[serde(default)]
    success_criteria: Vec<String>,
    #[serde(default)]
    budget: Option<AgentBudget>,
}

async fn append_inbox_messages(
    agent: &a4n_orchestrator::agent::AgentApplication,
    claimed: &AgentWorkerRun,
    messages: &mut Vec<ChatMessage>,
    loaded_message_ids: &mut HashSet<i64>,
) -> Result<(), AppError> {
    while let Some(row) = agent.claim_inbox_for_thread(claimed.run.thread_id).await? {
        let message_id = inbox_message_id(&row)?;
        if let Some(message_id) = message_id {
            if loaded_message_ids.contains(&message_id) {
                agent.consume_claimed_inbox(&row).await?;
                continue;
            }
            let message = agent
                .load_message_for_worker(claimed.run.thread_id, message_id)
                .await?;
            agent.consume_claimed_inbox(&row).await?;
            messages.push(chat_message_from_model(&message)?);
            loaded_message_ids.insert(message.id);
        } else {
            let (_, message) = agent.persist_claimed_inbox(claimed, &row).await?;
            messages.push(chat_message_from_model(&message)?);
            loaded_message_ids.insert(message.id);
        }
    }
    Ok(())
}

fn inbox_message_id(row: &a4n_core::models::agent::inbox::Model) -> Result<Option<i64>, AppError> {
    let Some(value) = row
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get("message_id"))
    else {
        return Ok(None);
    };
    if let Some(raw) = value.as_str() {
        return raw
            .parse::<i64>()
            .map(Some)
            .map_err(|error| AppError::InvalidState {
                current: format!("Agent inbox {} 的 message_id 无效: {error}", row.id),
                operation: "加载 Agent inbox message".into(),
            });
    }
    value
        .as_i64()
        .map(Some)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("Agent inbox {} 的 message_id 无效", row.id),
            operation: "加载 Agent inbox message".into(),
        })
}

async fn execute_tool_call(
    state: &AppState,
    agent: &a4n_orchestrator::agent::AgentApplication,
    claimed: &AgentWorkerRun,
    context: &AgentRunContext,
    executor: &RuntimeAgentToolExecutor,
    call: ToolCall,
    messages: &mut Vec<ChatMessage>,
) -> Result<(RunProgress, Vec<String>), AppError> {
    let descriptor = tool_descriptor(&call.name)?;
    let replay_policy = descriptor
        .agent_policy
        .as_ref()
        .map(|policy| enum_string(policy.replay_policy));
    let execution = agent
        .create_prepared_execution(WorkerExecutionInput {
            run_id: claimed.run.id,
            lease_token: claimed.lease_token.clone(),
            execution_key: format!("tool-call:{}", call.id),
            execution_kind: if call.name == "delegate" {
                "delegation".into()
            } else {
                "tool_call".into()
            },
            tool_name: Some(call.name.clone()),
            effect: Some(descriptor.effect.clone()),
            replay_policy,
            arguments_json: Some(call.arguments.clone()),
            resource_preconditions_json: None,
        })
        .await?;

    // A completed idempotent execution is a durable replay, not a reason to
    // call an external provider twice after a worker restart.
    if execution.status == "succeeded"
        && let Some(value) = execution.result_json.clone()
    {
        append_tool_observation(agent, claimed, &execution, &call, value, messages).await?;
        if context.profile_id == a4n_api_contract::agent::DIRECTOR_AGENT_CAPABILITY
            && call.name == "save_chapter_visual_plan_draft"
        {
            append_director_plan_saved_update(agent, claimed, execution.result_json.as_ref())
                .await?;
        }
        return Ok((
            RunProgress::Continue,
            discovered_tool_names(&call.name, &execution.result_json)?,
        ));
    }
    if matches!(
        execution.status.as_str(),
        "failed" | "rejected" | "cancelled"
    ) {
        let value = json!({"error": execution.error_message.clone().unwrap_or_else(|| "Agent 操作未完成".into())});
        append_tool_observation(agent, claimed, &execution, &call, value, messages).await?;
        return Ok((RunProgress::Continue, Vec::new()));
    }
    if execution.status == "outcome_unknown" {
        agent.wait_run(claimed, "reconcile").await?;
        return Ok((RunProgress::Released, Vec::new()));
    }

    if call.name == "delegate" {
        if should_request_approval(context, &call.name, &descriptor.effect)?
            && agent
                .approved_execution(claimed.run.thread_id, execution.id)
                .await?
                .is_none()
        {
            let operation = json!({
                "command": call.name,
                "arguments": call.arguments,
                "expected_effect": descriptor.effect,
            });
            let _ = agent
                .request_worker_approval(
                    claimed,
                    execution.id,
                    operation,
                    None,
                    Some(
                        chrono::Utc::now().naive_utc()
                            + chrono::Duration::seconds(WORKER_APPROVAL_TTL_SECONDS),
                    ),
                )
                .await?;
            return Ok((RunProgress::Released, Vec::new()));
        }
        if execution.status == "awaiting_user" {
            // An approved delegation is resumed through the same durable
            // execution state transition as every other guarded write.
            agent
                .update_worker_execution(WorkerExecutionUpdate {
                    run_id: claimed.run.id,
                    lease_token: claimed.lease_token.clone(),
                    execution_id: execution.id,
                    status: "dispatching".into(),
                    result_json: None,
                    result_summary: None,
                    result_refs_json: None,
                    error_message: None,
                })
                .await?;
        }
        let input = parse_arguments::<DelegateToolInput>(call.arguments.clone(), "delegate")?;
        let parent_budget: AgentBudget =
            serde_json::from_value(context.budget.clone()).map_err(|error| {
                AppError::InvalidState {
                    current: format!("Agent run {} budget_json 无效: {error}", claimed.run.id),
                    operation: "执行 Agent delegation".into(),
                }
            })?;
        enforce_delegation_limits(
            &context.delegations,
            context.run.id,
            context.delegation_depth,
            &parent_budget,
        )?;
        let child_budget = child_budget_within_parent(
            input.budget.unwrap_or_default(),
            &parent_budget,
            &context.usage,
        )?;
        let capability = input.capability.trim().to_string();
        let task = input.task.trim().to_string();
        if capability.is_empty() || task.is_empty() {
            return Err(AppError::InvalidArgument(
                "delegate 的 capability/task 不能为空".into(),
            ));
        }
        let project_id = input.project_id.or(context.scope.project_id);
        let chapter_id = input.chapter_id.or(context.scope.chapter_id);
        if let Some(project_id) = project_id {
            ensure_project_scope(&context.scope, project_id)?;
        }
        if let Some(chapter_id) = chapter_id {
            ensure_chapter_scope(&context.scope, chapter_id)?;
        }
        if !context.scope.allowed_capabilities.is_empty()
            && !context
                .scope
                .allowed_capabilities
                .iter()
                .any(|item| item == &capability)
        {
            return Err(AppError::Forbidden(format!(
                "Agent scope 不允许委派 capability: {capability}"
            )));
        }
        let binding = crate::actions::agent_domains::canonical_subagent_binding(
            &capability,
            project_id,
            chapter_id,
        )?;
        let mut child_scope = binding.scope();
        a4n_orchestrator::agent::inherit_subagent_allowlists(&context.scope, &mut child_scope)?;
        let child_thread = agent
            .ensure_canonical_subagent_thread(
                context.owner_user_id,
                a4n_orchestrator::agent::EnsureCanonicalSubagentThreadInput {
                    canonical_binding_key: binding.key,
                    capability: capability.clone(),
                    title: input.title,
                    scope: child_scope.clone(),
                    bound_resource: binding.bound_resource,
                    default_model_config_id: context.default_model_config_id,
                },
            )
            .await?;
        agent
            .append_worker_update(
                claimed,
                "delegation_started",
                "published",
                "正在启动子 Agent，等待其完成",
                Some(json!({ "capability": capability.clone(), "task": task.clone() })),
            )
            .await?;
        let _admission = state.lock_agent_admission().await;
        let result = agent
            .create_child_run(
                claimed,
                WorkerDelegationInput {
                    capability,
                    objective: task,
                    delegation_key: call.id,
                    child_thread_id: child_thread.id,
                    scope: child_scope,
                    success_criteria: input.success_criteria,
                    budget_json: serde_json::to_value(child_budget)?,
                    parent_execution_id: Some(execution.id),
                },
            )
            .await?;
        if let Some(snapshot) = state.agent_runtime_snapshot(claimed.run.id).await {
            state
                .store_agent_runtime_snapshot(result.child_run.id, Some(claimed.run.id), snapshot)
                .await;
        }
        tracing::info!(
            parent_run_id = claimed.run.id,
            delegation_id = result.delegation.id,
            child_run_id = result.child_run.id,
            "Agent child run started"
        );
        return Ok((RunProgress::Released, Vec::new()));
    }

    if should_request_approval(context, &call.name, &descriptor.effect)?
        && agent
            .approved_execution(claimed.run.thread_id, execution.id)
            .await?
            .is_none()
    {
        let operation = json!({
            "command": call.name,
            "arguments": call.arguments,
            "expected_effect": descriptor.effect,
        });
        let _ = agent
            .request_worker_approval(
                claimed,
                execution.id,
                operation,
                None,
                Some(
                    chrono::Utc::now().naive_utc()
                        + chrono::Duration::seconds(WORKER_APPROVAL_TTL_SECONDS),
                ),
            )
            .await?;
        return Ok((RunProgress::Released, Vec::new()));
    }

    if matches!(execution.status.as_str(), "awaiting_user" | "prepared") {
        // Approval releases the lease; reaching here means the operation was
        // approved and the run was claimed again.
        agent
            .update_worker_execution(WorkerExecutionUpdate {
                run_id: claimed.run.id,
                lease_token: claimed.lease_token.clone(),
                execution_id: execution.id,
                status: "dispatching".into(),
                result_json: None,
                result_summary: None,
                result_refs_json: None,
                error_message: None,
            })
            .await?;
    }
    agent
        .update_worker_execution(WorkerExecutionUpdate {
            run_id: claimed.run.id,
            lease_token: claimed.lease_token.clone(),
            execution_id: execution.id,
            status: "running".into(),
            result_json: None,
            result_summary: None,
            result_refs_json: None,
            error_message: None,
        })
        .await?;
    let approved = descriptor.effect == "read"
        || agent
            .approved_execution(claimed.run.thread_id, execution.id)
            .await?
            .is_some();
    let tool_call = executor.execute(
        context.owner_user_id,
        &context.scope,
        &call.name,
        call.arguments.clone(),
        approved,
    );
    let snapshot = state.agent_runtime_snapshot(claimed.run.id).await;
    let value = match async {
        match snapshot {
            Some(snapshot) => {
                crate::runtime_config::request_scope(
                    context.owner_user_id,
                    Some(snapshot),
                    tool_call,
                )
                .await
            }
            None => tool_call.await,
        }
    }
    .await
    {
        Ok(value) => {
            agent
                .update_worker_execution(WorkerExecutionUpdate {
                    run_id: claimed.run.id,
                    lease_token: claimed.lease_token.clone(),
                    execution_id: execution.id,
                    status: "succeeded".into(),
                    result_json: Some(value.clone()),
                    result_summary: Some("Agent tool completed".into()),
                    result_refs_json: None,
                    error_message: None,
                })
                .await?;
            if context.profile_id == a4n_api_contract::agent::DIRECTOR_AGENT_CAPABILITY
                && call.name == "save_chapter_visual_plan_draft"
            {
                append_director_plan_saved_update(agent, claimed, Some(&value)).await?;
            }
            value
        }
        Err(error) => {
            let error_text = error.to_string();
            agent
                .update_worker_execution(WorkerExecutionUpdate {
                    run_id: claimed.run.id,
                    lease_token: claimed.lease_token.clone(),
                    execution_id: execution.id,
                    status: "failed".into(),
                    result_json: None,
                    result_summary: None,
                    result_refs_json: None,
                    error_message: Some(error_text.clone()),
                })
                .await?;
            json!({"error": error_text})
        }
    };
    let discovered = discovered_tool_names(&call.name, &Some(value.clone()))?;
    append_tool_observation(agent, claimed, &execution, &call, value, messages).await?;
    Ok((RunProgress::Continue, discovered))
}

async fn append_director_plan_saved_update(
    agent: &a4n_orchestrator::agent::AgentApplication,
    claimed: &AgentWorkerRun,
    result: Option<&Value>,
) -> Result<(), AppError> {
    agent
        .append_worker_update(
            claimed,
            "director_plan_saved",
            "published",
            "导演编排草稿已保存，等待用户审核",
            Some(director_progress_payload(claimed, "saved", 3, result)),
        )
        .await
        .map(|_| ())
}

async fn append_tool_observation(
    agent: &a4n_orchestrator::agent::AgentApplication,
    claimed: &AgentWorkerRun,
    execution: &a4n_core::models::agent::execution::Model,
    call: &ToolCall,
    value: Value,
    messages: &mut Vec<ChatMessage>,
) -> Result<(), AppError> {
    agent
        .append_tool_message(
            claimed,
            execution.id,
            &call.name,
            Some(&call.id),
            value.clone(),
        )
        .await?;
    let mut tool_message = ChatMessage::new(MessageRole::Tool, serde_json::to_string(&value)?);
    tool_message.tool_call_id = Some(call.id.clone());
    messages.push(tool_message);
    Ok(())
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
        "autonomous" => Ok(tool_requires_approval(tool_name, effect)),
        mode => Err(AppError::InvalidState {
            current: format!("未知 Agent execution_mode: {mode}"),
            operation: "执行 Agent 写工具".into(),
        }),
    }
}

async fn persist_usage(
    agent: &a4n_orchestrator::agent::AgentApplication,
    claimed: &AgentWorkerRun,
    usage: &Value,
    steps: u32,
    model_calls: u32,
    tool_calls: u32,
    tokens: u64,
) -> Result<(), AppError> {
    let mut usage = usage.clone();
    usage["steps_used"] = json!(steps);
    usage["model_calls_used"] = json!(model_calls);
    usage["tool_calls_used"] = json!(tool_calls);
    usage["tokens_used"] = json!(tokens);
    agent
        .update_run_usage(WorkerUsage {
            run_id: claimed.run.id,
            lease_token: claimed.lease_token.clone(),
            usage_json: usage,
        })
        .await?;
    Ok(())
}

fn with_agent_system_prompt(
    mut messages: Vec<ChatMessage>,
    context: &AgentRunContext,
) -> Vec<ChatMessage> {
    let role = if context.run.parent_run_id.is_some() {
        "你是由上级 Agent 委派的领域子 Agent；专注完成当前能力域目标，完成后提交可供上级验收的真实结果。"
    } else {
        "你是掌管项目全局的主 Agent；可以自主定位资源、调用工具和委派领域子 Agent，并负责检查子 Agent 回传结果后再向用户交付。"
    };
    let memories = context
        .memories
        .iter()
        .map(|memory| {
            format!(
                "- {}/{}/{}: {}",
                memory.scope_kind, memory.scope_key, memory.memory_key, memory.content
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let domain_rules = if context.profile_id == a4n_api_contract::agent::DIRECTOR_AGENT_CAPABILITY {
        "导演编排约束：只读取当前绑定章节及其项目上下文；输出完整的 ChapterVisualPlanV2；只能通过 save_chapter_visual_plan_draft 保存 collaborative 草稿；不得确认、丢弃或锁定方案，用户是最终审核者；只有保存工具真实成功后才能声称草稿已保存。"
    } else {
        ""
    };
    let prompt = format!(
        "你是 A4N 项目 Agent。{role}\n\
当前能力域：{}\n\
{}\n\
当前目标：{}\n\
执行模式：{}\n\
资源范围：{}\n\
父 Run：{}\n\
有效记忆：\n{}\n\
请根据目标自主规划，不要受固定步骤束缚。工具结果和领域事实源是唯一可信依据；没有实际执行成功时不得声称完成。严格遵守资源范围、权限、版本前置条件、审批与预算边界。需要专业领域能力时可委派；主 Agent 必须读取并验收子 Agent 的回传内容。完成后用简洁中文说明实际结果、关键变更和未解决事项。",
        context.profile_id,
        domain_rules,
        context.goal.as_deref().unwrap_or("未提供显式目标"),
        context.execution_mode,
        serde_json::to_string(&context.scope).expect("AgentScope must serialize"),
        context
            .run
            .parent_run_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "无（主 Agent）".into()),
        if memories.is_empty() {
            "无"
        } else {
            &memories
        },
    );
    messages.insert(0, ChatMessage::system(prompt));
    messages
}

fn director_progress_payload(
    claimed: &AgentWorkerRun,
    phase: &str,
    current: u32,
    result: Option<&Value>,
) -> Value {
    let mut payload = json!({
        "phase": phase,
        "current": current,
        "total": 3,
    });
    if claimed.thread.resource_type.as_deref() == Some("chapter")
        && let Some(chapter_id) = claimed
            .thread
            .resource_id
            .as_deref()
            .and_then(|value| value.parse::<i64>().ok())
    {
        payload["chapter_id"] = json!(chapter_id);
    }
    if let Some(record_id) = result
        .and_then(|value| value.get("record"))
        .and_then(|record| record.get("id"))
    {
        payload["record_id"] = record_id.clone();
    }
    payload
}

fn chat_messages(context: &AgentRunContext) -> Result<Vec<ChatMessage>, AppError> {
    context
        .messages
        .iter()
        .map(chat_message_from_model)
        .collect()
}

fn chat_message_from_model(
    message: &a4n_core::models::agent::message::Model,
) -> Result<ChatMessage, AppError> {
    let role = match message.role.as_str() {
        "system" => MessageRole::System,
        "user" => MessageRole::User,
        "assistant" => MessageRole::Assistant,
        "tool" => MessageRole::Tool,
        other => {
            return Err(AppError::InvalidState {
                current: format!("未知 Agent message role: {other}"),
                operation: "构建 Agent model context".into(),
            });
        }
    };
    let mut result = ChatMessage::new(role, message.content.clone());
    if let Some(payload) = message.payload_json.as_ref() {
        if let Some(calls) = payload.get("tool_calls") {
            result.tool_calls = Some(serde_json::from_value(calls.clone()).map_err(|error| {
                AppError::InvalidState {
                    current: format!("Agent message {} tool_calls 无效: {error}", message.id),
                    operation: "构建 Agent model context".into(),
                }
            })?);
        }
        result.tool_call_id = payload
            .get("tool_call_id")
            .and_then(Value::as_str)
            .map(str::to_string);
    }
    Ok(result)
}

fn available_runtime_tools(
    capability: &str,
    scope: &AgentScope,
    discovered_tools: &[String],
) -> Result<Vec<Value>, AppError> {
    let mut names = crate::actions::agent_domains::tools_for_capability(capability)
        .iter()
        .map(|name| (*name).to_string())
        .collect::<Vec<_>>();
    names.retain(|name| ensure_scope_allows_tool(scope, name).is_ok());
    for name in discovered_tools {
        if !names.iter().any(|known| known == name) && ensure_scope_allows_tool(scope, name).is_ok()
        {
            names.push(name.clone());
        }
    }
    names
        .iter()
        .map(|name| {
            let descriptor = tool_descriptor(name)?;
            Ok(json!({
                "type": "function",
                "function": {
                    "name": descriptor.name,
                    "description": descriptor.summary,
                    "parameters": descriptor.args_schema,
                }
            }))
        })
        .collect()
}

fn discovered_tool_names(tool_name: &str, result: &Option<Value>) -> Result<Vec<String>, AppError> {
    let Some(result) = result else {
        return Ok(Vec::new());
    };
    if result.get("error").is_some() {
        return Ok(Vec::new());
    }
    match tool_name {
        "search_agent_tools" => serde_json::from_value::<AgentToolSearchResult>(result.clone())
            .map(|found| found.tools.into_iter().map(|tool| tool.name).collect())
            .map_err(|error| AppError::InvalidState {
                current: format!("Agent 工具 {tool_name} 返回的工具目录无效: {error}"),
                operation: "恢复 Agent 工具发现结果".into(),
            }),
        "describe_agent_tool" => serde_json::from_value::<AgentToolDescriptor>(result.clone())
            .map(|tool| vec![tool.name])
            .map_err(|error| AppError::InvalidState {
                current: format!("Agent 工具 {tool_name} 返回的工具描述无效: {error}"),
                operation: "恢复 Agent 工具发现结果".into(),
            }),
        _ => Ok(Vec::new()),
    }
}

fn extend_discovered_tools(
    discovered_tools: &mut Vec<String>,
    names: Vec<String>,
    scope: &AgentScope,
) -> Result<(), AppError> {
    for name in names {
        let name = name.trim();
        if name.is_empty() || ensure_scope_allows_tool(scope, name).is_err() {
            continue;
        }
        // Re-resolve every discovered name through the canonical descriptor.
        // The model-facing result is not trusted as an executable schema.
        tool_descriptor(name)?;
        if !discovered_tools.iter().any(|known| known == name) {
            discovered_tools.push(name.to_string());
        }
    }
    Ok(())
}

fn usage_object(value: &Value) -> Result<Value, AppError> {
    if value.is_object() {
        Ok(value.clone())
    } else {
        Err(AppError::InvalidState {
            current: "Agent usage_json 不是对象".into(),
            operation: "读取 Agent usage".into(),
        })
    }
}

fn json_u32(value: &Value, key: &str) -> Result<u32, AppError> {
    u32::try_from(json_u64(value, key)?).map_err(|_| AppError::InvalidState {
        current: format!("Agent usage.{key} 超出 u32"),
        operation: "读取 Agent run".into(),
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
        .orchestrator()
        .agent()
        .ensure_main_thread(user_id, input)
        .await
}

pub async fn create_thread(
    state: &AppState,
    user_id: i64,
    input: CreateAgentThreadInput,
) -> Result<AgentThreadView, AppError> {
    state
        .orchestrator()
        .agent()
        .create_thread(user_id, input)
        .await
}

pub async fn fork_thread(
    state: &AppState,
    user_id: i64,
    input: ForkAgentThreadInput,
) -> Result<AgentThreadView, AppError> {
    state
        .orchestrator()
        .agent()
        .fork_thread(user_id, input)
        .await
}

pub async fn update_thread(
    state: &AppState,
    user_id: i64,
    input: UpdateAgentThreadInput,
) -> Result<AgentThreadView, AppError> {
    state
        .orchestrator()
        .agent()
        .update_thread(user_id, input)
        .await
}

pub async fn archive_thread(
    state: &AppState,
    user_id: i64,
    input: ArchiveAgentThreadInput,
) -> Result<AgentThreadView, AppError> {
    state
        .orchestrator()
        .agent()
        .archive_thread(user_id, input)
        .await
}

pub async fn list_threads(
    state: &AppState,
    user_id: i64,
    input: ListAgentThreadsInput,
) -> Result<Vec<AgentThreadView>, AppError> {
    state
        .orchestrator()
        .agent()
        .list_threads(user_id, input)
        .await
}

pub async fn show_thread(
    state: &AppState,
    user_id: i64,
    thread_id: &str,
) -> Result<AgentThreadSnapshot, AppError> {
    state
        .orchestrator()
        .agent()
        .show_thread(user_id, thread_id)
        .await
}

pub async fn list_messages(
    state: &AppState,
    user_id: i64,
    input: ListAgentMessagesInput,
) -> Result<Vec<a4n_api_contract::agent::AgentMessageView>, AppError> {
    state
        .orchestrator()
        .agent()
        .list_messages(user_id, input)
        .await
}

pub async fn show_run(
    state: &AppState,
    user_id: i64,
    run_id: &str,
) -> Result<AgentRunView, AppError> {
    state.orchestrator().agent().show_run(user_id, run_id).await
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
    let accepted = state
        .orchestrator()
        .agent()
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
        .orchestrator()
        .agent()
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
    let accepted = state
        .orchestrator()
        .agent()
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
        .orchestrator()
        .agent()
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
    let result = state
        .orchestrator()
        .agent()
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
    state.orchestrator().agent().events(user_id, input).await
}

pub async fn search_tools(
    state: &AppState,
    input: SearchAgentToolsInput,
) -> Result<AgentToolSearchResult, AppError> {
    state.orchestrator().agent().search_tools(input)
}

pub fn describe_tool(
    state: &AppState,
    input: DescribeAgentToolInput,
) -> Result<AgentToolDescriptor, AppError> {
    state.orchestrator().agent().describe_tool(input)
}

pub async fn list_memories(
    state: &AppState,
    user_id: i64,
    scope_kind: Option<AgentMemoryScopeKind>,
    scope_id: Option<String>,
) -> Result<Vec<AgentMemoryView>, AppError> {
    state
        .orchestrator()
        .agent()
        .list_memories(user_id, scope_kind, scope_id)
        .await
}

pub async fn create_memory(
    state: &AppState,
    user_id: i64,
    input: AgentMemoryInput,
) -> Result<AgentMemoryView, AppError> {
    state
        .orchestrator()
        .agent()
        .create_memory(user_id, input)
        .await
}

pub async fn retract_memory(
    state: &AppState,
    user_id: i64,
    input: RetractAgentMemoryInput,
) -> Result<AgentMemoryView, AppError> {
    state
        .orchestrator()
        .agent()
        .retract_memory(user_id, input)
        .await
}

pub async fn resolve_approval(
    state: &AppState,
    user_id: i64,
    input: ResolveAgentApprovalInput,
) -> Result<AgentApprovalView, AppError> {
    let result = state
        .orchestrator()
        .agent()
        .resolve_approval(user_id, input)
        .await?;
    state.trigger_agent_worker();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovered_manifest_tool_is_added_to_next_model_schema() {
        let scope = AgentScope::default();
        let tools = available_runtime_tools(
            a4n_api_contract::agent::MAIN_AGENT_CAPABILITY,
            &scope,
            &["create_project".into()],
        )
        .expect("manifest tool should be serializable");
        assert!(tools.iter().any(|tool| {
            tool.pointer("/function/name") == Some(&Value::String("create_project".into()))
        }));
    }

    #[test]
    fn usage_counters_reject_malformed_values_instead_of_resetting_to_zero() {
        assert_eq!(
            json_u32(&json!({}), "steps_used").expect("missing is zero"),
            0
        );
        assert!(json_u32(&json!({"steps_used": "1"}), "steps_used").is_err());
        assert!(json_u64(&json!({"tokens_used": -1}), "tokens_used").is_err());
    }

    #[test]
    fn generic_subagent_scope_can_add_discovered_manifest_tools() {
        let mut discovered = Vec::new();
        extend_discovered_tools(
            &mut discovered,
            vec!["create_project".into(), "delete_project".into()],
            &AgentScope::default(),
        )
        .expect("generic sub-agent should accept manifest-discovered tools");
        assert_eq!(discovered, ["create_project", "delete_project"]);
        let tools = available_runtime_tools("general", &AgentScope::default(), &discovered)
            .expect("generic discovered tools should be serializable");
        assert!(tools.iter().any(|tool| {
            tool.pointer("/function/name") == Some(&Value::String("create_project".into()))
        }));
    }

    #[test]
    fn discovered_tools_keep_scope_and_manifest_policy_boundaries() {
        let scope = AgentScope {
            allowed_tools: vec!["list_projects".into()],
            ..AgentScope::default()
        };
        let mut discovered = Vec::new();
        extend_discovered_tools(
            &mut discovered,
            vec!["create_project".into(), "create_llm_config".into()],
            &scope,
        )
        .expect("out-of-scope tools are ignored");
        assert!(discovered.is_empty());
        let tools =
            available_runtime_tools(a4n_api_contract::agent::MAIN_AGENT_CAPABILITY, &scope, &[])
                .expect("scoped static tools should be serializable");
        assert!(tools.iter().all(|tool| {
            tool.pointer("/function/name") == Some(&Value::String("list_projects".into()))
        }));
        let mut unrestricted = Vec::new();
        assert!(
            extend_discovered_tools(
                &mut unrestricted,
                vec!["create_llm_config".into()],
                &AgentScope::default(),
            )
            .is_err()
        );
    }

    #[test]
    fn dynamic_destructive_tools_and_nested_resource_ids_are_fenced() {
        assert!(tool_requires_approval("delete_project", "destructive"));
        assert!(tool_requires_approval("run_canvas_graph", "write"));
        assert!(!tool_requires_approval("create_project", "write"));

        let scope = AgentScope {
            project_id: Some(7),
            chapter_id: Some(11),
            ..AgentScope::default()
        };
        let descriptor = a4n_api_contract::command_manifest::command_descriptor("update_project")
            .expect("manifest descriptor");
        ensure_argument_scope(
            &scope,
            &descriptor,
            &json!({"projectId": 7, "input": {"title": "ok"}}),
        )
        .expect("matching project scope");
        assert!(
            ensure_argument_scope(
                &scope,
                &descriptor,
                &json!({"projectId": 8, "input": {"title": "blocked"}}),
            )
            .is_err()
        );
        assert!(
            ensure_argument_scope(&scope, &descriptor, &json!({"input": {"chapter_id": 12}}),)
                .is_err()
        );
        ensure_argument_scope(
            &scope,
            &descriptor,
            &json!({"project_ids": ["7"], "input": {"chapterIds": [11]}}),
        )
        .expect("plural and string resource ids within scope");
        assert!(
            ensure_argument_scope(
                &scope,
                &descriptor,
                &json!({"runtime_config_snapshot": {"providerId": 999}}),
            )
            .is_err()
        );
        assert!(
            ensure_argument_scope(&scope, &descriptor, &json!({"title": "must be scoped"}),)
                .is_err()
        );
        assert!(ensure_argument_scope(&scope, &descriptor, &json!({})).is_err());
        assert!(
            ensure_argument_scope(&scope, &descriptor, &json!({"project_ids": ["8"]}),).is_err()
        );
    }
}
