use super::*;
use a4n_api_contract::chapter_visual_plan::{
    ChapterVisualPlanView, ConfirmedChapterVisualPlanView,
};

#[derive(Clone)]
pub(crate) struct RuntimeAgentToolExecutor {
    state: AgentRuntimeState,
    pub(crate) worker_request: WorkerEnvelope<AgentRunPayload>,
}

impl RuntimeAgentToolExecutor {
    pub(crate) fn new(
        state: AgentRuntimeState,
        worker_request: WorkerEnvelope<AgentRunPayload>,
    ) -> Self {
        Self {
            state,
            worker_request,
        }
    }

    /// Execute a validated Agent tool through the owning Host.
    pub(crate) async fn execute(
        &self,
        claimed: &AgentWorkerRun,
        context: &AgentRunContext,
        name: &str,
        arguments: Value,
        approved_write: bool,
        execution: Option<&AgentExecutionSnapshot>,
    ) -> Result<Value, AppError> {
        let descriptor = tool_descriptor(name)?;
        ensure_agent_scope_allows_tool(&context.scope, name)?;
        ensure_argument_scope(&context.scope, &descriptor, &arguments)?;
        ensure_argument_resource_scope(
            self.state.host(),
            context.owner_user_id,
            &context.scope,
            &arguments,
        )
        .await?;
        if a4n_api_contract::agent::agent_tool_requires_user_approval(name, &descriptor.effect)
            && !approved_write
        {
            return Err(AppError::Forbidden(format!(
                "Agent 工具 {name} 是写操作，必须先获得用户审批"
            )));
        }

        if task_wait::is_wait(name) {
            let execution = execution.ok_or_else(|| AppError::InvalidState {
                current: format!("Agent wait 工具 {name} 缺少 durable execution"),
                operation: "读取 Agent 外部任务等待".into(),
            })?;
            let snapshot = self.state.agent_runtime_snapshot(claimed.run.id).await;
            return self
                .state
                .host()
                .read_external_task_wait(
                    &AgentWorkerExternalTaskWaitSnapshotWire {
                        run_id: claimed.run.id,
                        owner_user_id: context.owner_user_id,
                        scope: context.scope.clone(),
                        execution_id: execution.id,
                        tool_name: Some(name.to_string()),
                        arguments_json: Some(arguments),
                        duration_exhausted: false,
                    },
                    snapshot.as_ref(),
                )
                .await;
        }

        match name {
            "search_agent_tools" => {
                let mut input = parse_arguments::<SearchAgentToolsInput>(arguments, name)?;
                a4n_api_contract::agent::scope_agent_tool_search(&context.profile_id, &mut input);
                serde_json::to_value(
                    self.state
                        .host()
                        .search_agent_tools(&AgentWorkerToolSearchRequest { input })
                        .await?,
                )
                .map_err(AppError::from)
            }
            "search_agent_skills" => serde_json::to_value(
                self.state
                    .search_agent_skills(
                        claimed,
                        parse_arguments::<SearchAgentSkillsInput>(arguments, name)?,
                    )
                    .await?,
            )
            .map_err(AppError::from),
            "load_agent_skill" => serde_json::to_value(
                self.state
                    .load_agent_skill(
                        claimed,
                        parse_arguments::<LoadAgentSkillInput>(arguments, name)?,
                    )
                    .await?,
            )
            .map_err(AppError::from),
            "describe_agent_tool" => {
                let input = parse_arguments::<DescribeAgentToolInput>(arguments, name)?;
                ensure_agent_scope_allows_tool(&context.scope, &input.name)?;
                serde_json::to_value(
                    self.state
                        .host()
                        .describe_agent_tool(&AgentWorkerToolDescribeRequest { input })
                        .await?,
                )
                .map_err(AppError::from)
            }
            "read_agent_memory" => {
                let input = parse_arguments::<a4n_api_contract::agent::ReadAgentMemoryInput>(
                    arguments, name,
                )?;
                serde_json::to_value(self.state.read_agent_memory(claimed, input).await?)
                    .map_err(AppError::from)
            }
            "read_agent_history" => {
                let input = parse_arguments::<a4n_api_contract::agent::ReadAgentHistoryInput>(
                    arguments, name,
                )?;
                serde_json::to_value(self.state.read_agent_history(claimed, input).await?)
                    .map_err(AppError::from)
            }
            "read_agent_execution_result" => {
                let input = parse_arguments::<
                    a4n_api_contract::agent::ReadAgentExecutionResultInput,
                >(arguments, name)?;
                serde_json::to_value(
                    self.state
                        .read_agent_execution_result(claimed, input)
                        .await?,
                )
                .map_err(AppError::from)
            }
            // Durable child creation is handled by execute_tool_call.
            "delegate" => unreachable!("delegate handled by Agent worker"),
            "review_agent_delegation" => {
                let input = parse_arguments::<
                    HostInput<a4n_api_contract::agent::ReviewAgentDelegationInput>,
                >(arguments, name)?
                .input;
                serde_json::to_value(
                    self.state
                        .review_agent_delegation(
                            &self.worker_request,
                            context.owner_user_id,
                            Some(context.run.id),
                            input,
                        )
                        .await?,
                )
                .map_err(AppError::from)
            }
            _ => {
                self.state
                    .dispatch_agent_host_command(
                        context.owner_user_id,
                        &context.scope,
                        name.to_string(),
                        arguments,
                        &self.worker_request,
                        execution.map(|execution| execution.id),
                    )
                    .await
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HostInput<T> {
    input: T,
}

pub fn tool_descriptor(
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
pub fn ensure_argument_scope(
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
        && !matches!(
            descriptor.name.as_str(),
            "web_search" | "web_fetch" | "review_agent_delegation"
        )
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
    host: &dyn AgentRuntimeHost,
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
    let mut source_chapter_ids = Vec::new();
    collect_argument_chapter_targets(arguments, &mut chapter_ids, &mut source_chapter_ids);
    collect_scope_ids(arguments, &mut project_ids, &mut Vec::new());
    if chapter_ids.is_empty() && source_chapter_ids.is_empty() {
        return Ok(());
    }
    for requested_project in project_ids {
        ensure_project_scope(scope, requested_project)?;
    }
    host.verify_agent_chapters(&AgentWorkerChapterScopeRequest {
        user_id,
        project_id,
        chapter_ids,
        source_chapter_ids,
    })
    .await
}

pub fn collect_argument_chapter_targets(
    value: &Value,
    chapters: &mut Vec<i64>,
    sources: &mut Vec<i64>,
) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                let key = key.to_ascii_lowercase();
                if matches!(
                    key.as_str(),
                    "source_chapter_id"
                        | "source_chapter_ids"
                        | "sourcechapterid"
                        | "sourcechapterids"
                ) {
                    collect_numeric_ids(value, sources);
                } else {
                    if is_chapter_scope_key(&key) {
                        collect_numeric_ids(value, chapters);
                    }
                    collect_argument_chapter_targets(value, chapters, sources);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_argument_chapter_targets(item, chapters, sources);
            }
        }
        _ => {}
    }
}

pub(crate) async fn ensure_agent_package_enabled(
    state: &AgentRuntimeState,
) -> Result<(), AppError> {
    if state.host().package_execution_enabled().await? {
        Ok(())
    } else {
        Err(AppError::Forbidden("Agent package 当前不可用".into()))
    }
}

pub fn collect_scope_ids(value: &Value, project_ids: &mut Vec<i64>, chapter_ids: &mut Vec<i64>) {
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

pub fn ensure_project_scope(scope: &AgentScope, project_id: i64) -> Result<(), AppError> {
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

pub fn ensure_chapter_scope(scope: &AgentScope, chapter_id: i64) -> Result<(), AppError> {
    if chapter_id <= 0 {
        return Err(AppError::InvalidArgument("chapterId 无效".into()));
    }
    if scope.chapter_id.is_some_and(|scoped| scoped != chapter_id) {
        return Err(AppError::Forbidden(format!(
            "Agent scope 仅允许 chapter_id={}（第{}章），不能访问 chapter_id={chapter_id}",
            scope.chapter_id.expect("checked Some"),
            scope
                .chapter_number
                .map_or_else(|| "业务序号未知".to_string(), |number| format!("{number}"))
        )));
    }
    Ok(())
}

pub fn parse_arguments<T: DeserializeOwned>(arguments: Value, name: &str) -> Result<T, AppError> {
    serde_json::from_value(arguments)
        .map_err(|error| AppError::InvalidArgument(format!("Agent 工具 {name} 参数无效: {error}")))
}

pub fn editor_request_for_call(
    context: &AgentRunContext,
    execution: &AgentExecutionSnapshot,
    call: &ToolCall,
) -> Result<a4n_api_contract::agent::AgentEditorRequest, AppError> {
    ensure_agent_scope_allows_tool(&context.scope, &call.name)?;
    ensure_argument_scope(
        &context.scope,
        &tool_descriptor(&call.name)?,
        &call.arguments,
    )?;
    let source = context
        .editor_source
        .clone()
        .ok_or_else(|| AppError::InvalidArgument("Turn 缺少编辑器来源".into()))?;
    let operation = match call.name.as_str() {
        "control_editor_view" => a4n_api_contract::agent::AgentEditorOperation::Control {
            input: parse_arguments(call.arguments.clone(), &call.name)?,
        },
        "inspect_editor_target" => {
            if call.arguments != json!({}) {
                return Err(AppError::InvalidArgument(
                    "inspect_editor_target 不接受参数".into(),
                ));
            }
            a4n_api_contract::agent::AgentEditorOperation::Inspect
        }
        "read_editor_draft" => a4n_api_contract::agent::AgentEditorOperation::Read {
            input: parse_arguments(call.arguments.clone(), &call.name)?,
        },
        "save_editor_draft" => a4n_api_contract::agent::AgentEditorOperation::Save {
            input: parse_arguments(call.arguments.clone(), &call.name)?,
        },
        "edit_chapter_visual_plan_draft" => {
            let input: HostInput<
                a4n_api_contract::chapter_visual_plan::EditChapterVisualPlanDraftInput,
            > = parse_arguments(call.arguments.clone(), &call.name)?;
            a4n_api_contract::agent::AgentEditorOperation::PrepareWrite {
                expected_editor_revision: input.input.editor_revision.ok_or_else(|| {
                    AppError::InvalidArgument(
                        "编辑器内修改必须提供 inspect_editor_target 返回的 editor_revision".into(),
                    )
                })?,
                record_id: Some(input.input.record_id),
                record_revision: Some(input.input.expected_revision),
            }
        }
        "save_chapter_visual_plan_draft" => {
            let input: HostInput<
                a4n_api_contract::chapter_visual_plan::SaveChapterVisualPlanDraftInput,
            > = parse_arguments(call.arguments.clone(), &call.name)?;
            a4n_api_contract::agent::AgentEditorOperation::PrepareWrite {
                expected_editor_revision: input.input.editor_revision.ok_or_else(|| {
                    AppError::InvalidArgument(
                        "编辑器内保存必须提供 inspect_editor_target 返回的 editor_revision".into(),
                    )
                })?,
                record_id: input.input.record_id,
                record_revision: input.input.expected_revision,
            }
        }
        _ => return Err(AppError::InvalidArgument("不是编辑器工具".into())),
    };
    Ok(a4n_api_contract::agent::AgentEditorRequest {
        execution_id: execution.id.to_string(),
        source,
        operation,
    })
}

pub fn tool_result_refs(
    name: &str,
    value: &Value,
) -> Result<Vec<a4n_api_contract::agent::AgentResourceRef>, AppError> {
    use a4n_api_contract::agent::AgentResourceRef;
    if observation::error_value(value).is_some() {
        return Ok(Vec::new());
    }
    let mut refs = Vec::new();
    let mut push_ref = |resource: AgentResourceRef| {
        refs.retain(|previous: &AgentResourceRef| {
            previous.kind != resource.kind || previous.id != resource.id
        });
        refs.push(resource);
    };
    if let Some(result_refs) = value.get("result_refs").filter(|value| !value.is_null()) {
        for resource in serde_json::from_value::<Vec<AgentResourceRef>>(result_refs.clone())? {
            push_ref(resource);
        }
    }
    if let Some(resource) = value.get("resource").filter(|value| !value.is_null()) {
        push_ref(serde_json::from_value(resource.clone())?);
    }
    match name {
        "list_optimize_turns" => {
            for turn in serde_json::from_value::<
                Vec<a4n_api_contract::optimize::OptimizationTurnSummary>,
            >(value.clone())?
            {
                if let Some(reply) = turn.final_reply {
                    push_ref(AgentResourceRef {
                        kind: "optimization_turn".into(),
                        id: turn.id.to_string(),
                        revision: Some(a4n_domain::source_text_hash(&reply)),
                        hash: None,
                        label: None,
                    });
                }
            }
        }
        "list_optimize_patch_sets" => {
            for patch in serde_json::from_value::<
                Vec<a4n_api_contract::optimize::OptimizationPatchSetSummary>,
            >(value.clone())?
            {
                push_ref(AgentResourceRef {
                    kind: "optimization_patch_set".into(),
                    id: patch.id.to_string(),
                    revision: Some(patch.updated_at),
                    hash: Some(patch.base_content_hash),
                    label: Some(patch.status),
                });
            }
        }
        "get_canvas_graph" | "save_canvas_graph" => {
            let saved: a4n_api_contract::canvas::CanvasGraphDocument =
                serde_json::from_value(value.clone())?;
            push_ref(AgentResourceRef {
                kind: "canvas_graph".into(),
                id: saved.graph.id.to_string(),
                revision: Some(saved.graph.version.to_string()),
                hash: None,
                label: Some(saved.graph.name),
            });
        }
        "apply_optimization_patches" => {
            let saved: a4n_api_contract::optimize::ApplyOptimizationPatchesResponse =
                serde_json::from_value(value.clone())?;
            push_ref(AgentResourceRef {
                kind: "chapter".into(),
                id: saved.chapter_id.to_string(),
                revision: None,
                hash: None,
                label: Some("已应用正文补丁".into()),
            });
        }
        "save_chapter_visual_plan_draft" | "confirm_chapter_visual_plan" => {
            let (saved, label) = if name == "confirm_chapter_visual_plan" {
                let confirmed: ConfirmedChapterVisualPlanView =
                    serde_json::from_value(value.clone())?;
                (confirmed.visual_plan, "已确认视觉方案")
            } else {
                let saved: ChapterVisualPlanView = serde_json::from_value(value.clone())?;
                (saved, "视觉方案草稿")
            };
            push_ref(AgentResourceRef {
                kind: "chapter_visual_plan".into(),
                id: saved.record.id.to_string(),
                revision: Some(saved.record.revision.to_string()),
                hash: Some(saved.record.plan_sha256),
                label: Some(label.into()),
            });
        }
        "run_canvas_node" | "run_canvas_graph" => {
            let run: a4n_api_contract::canvas::CanvasRunSummary =
                serde_json::from_value(value.clone())?;
            push_ref(AgentResourceRef {
                kind: "workflow_run".into(),
                id: run.workflow_run_id.to_string(),
                revision: None,
                hash: None,
                label: Some(run.status),
            });
        }
        "get_generation_task_summary" | "wait_generation_task" => {
            let summary: a4n_api_contract::generation_tasks::GenerationTaskSummary =
                serde_json::from_value(value.clone())?;
            if summary.handle.status == a4n_service_protocol::GenerationTaskStatus::Completed {
                push_ref(AgentResourceRef {
                    kind: "generation_task".into(),
                    id: summary.handle.task_id,
                    revision: Some(summary.handle.attempt.to_string()),
                    hash: None,
                    label: Some("completed".into()),
                });
                if let Some(result) = summary.result_ref {
                    push_ref(AgentResourceRef {
                        kind: result.kind,
                        id: result.id,
                        revision: Some(result.revision),
                        hash: None,
                        label: None,
                    });
                }
            }
        }
        "get_workflow_run_detail" | "wait_workflow_run" => {
            let detail: a4n_api_contract::workflow::WorkflowRunDetailSummary =
                serde_json::from_value(value.clone())?;
            if detail.run.status == "completed" {
                push_ref(AgentResourceRef {
                    kind: "workflow_run".into(),
                    id: detail.run.id.to_string(),
                    revision: None,
                    hash: None,
                    label: Some(detail.run.status),
                });
            }
        }
        _ => {}
    }
    Ok(refs)
}

pub fn should_request_approval(
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
        // edit_article only persists a reviewable patch proposal. The chapter
        // write boundary remains apply_optimization_patches.
        _ if tool_name == "edit_article" => Ok(false),
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
