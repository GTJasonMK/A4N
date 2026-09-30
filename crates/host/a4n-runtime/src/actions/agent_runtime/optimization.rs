use super::*;

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
        "locate_writing_target" => {
            let input = parse_arguments::<LocateWritingTargetInput>(arguments, &command)?;
            serde_json::to_value(locate_writing_target(state, user_id, &scope, input).await?)
                .map_err(AppError::from)
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

#[derive(Clone)]
pub(crate) struct RuntimeAgentToolExecutor {
    state: AppState,
    worker_request: WorkerEnvelope<AgentRunPayload>,
}

impl RuntimeAgentToolExecutor {
    pub(crate) fn new(state: AppState, worker_request: WorkerEnvelope<AgentRunPayload>) -> Self {
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
                        .agent_worker_memory_read_application()
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
                        .agent_worker_execution_result_read_application()
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
            "inspect_editor_target"
            | "read_editor_draft"
            | "save_editor_draft"
            | "control_editor_view" => {
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
    let mut source_chapter_ids = Vec::new();
    a4n_agent_runtime::engine::collect_argument_chapter_targets(
        arguments,
        &mut chapter_ids,
        &mut source_chapter_ids,
    );
    collect_scope_ids(arguments, &mut project_ids, &mut Vec::new());
    if chapter_ids.is_empty() && source_chapter_ids.is_empty() {
        return Ok(());
    }
    for requested_project in project_ids {
        ensure_project_scope(scope, requested_project)?;
    }
    state
        .agent_resource_access_application()
        .verify_chapters_in_project(user_id, project_id, &chapter_ids, &source_chapter_ids)
        .await
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
