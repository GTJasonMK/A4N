//! Agent 消息、delegation、memory、tool 与 approval 操作。

use super::*;

impl AgentApplication {
    pub(crate) fn describe_tool(
        &self,
        input: a4n_api_contract::agent::DescribeAgentToolInput,
    ) -> Result<AgentToolDescriptor, AppError> {
        let name = required_text(input.name, "name")?;
        let (tools, _) = self.search_tool_descriptors(SearchAgentToolsInput {
            query: Some(name.clone()),
            domain: None,
            effect: None,
            capability: None,
            limit: Some(AGENT_TOOLS_MAX_LIMIT),
        })?;
        tools
            .into_iter()
            .find(|tool| tool.name == name)
            .ok_or_else(|| AppError::NotFound(format!("Agent tool 不存在或未公开: {name}")))
    }

    pub(crate) async fn send_message(
        &self,
        user_id: i64,
        mut input: SendAgentMessageInput,
        runtime_snapshot_required: bool,
        require_runtime_snapshot_for_new_run: bool,
    ) -> Result<AgentMessageAccepted, AppError> {
        let thread_id = parse_id(&input.thread_id, "thread_id")?;
        let thread = self.owned_thread(user_id, thread_id).await?;
        let mut scope: AgentScope =
            from_json(thread.scope_json.clone(), "agent_thread.scope_json")?;
        self.hydrate_scope_chapter_number(&mut scope).await?;
        let editor_source = match input.editor_source.take() {
            Some(source) => {
                Some(canonicalize_editor_source(self.executor(), source, &thread).await?)
            }
            None => None,
        };
        let client_message_id = required_text(input.client_message_id, "client_message_id")?;
        let content = required_text(input.content, "content")?;
        validate_optional_positive_id(input.model_config_id, "model_config_id")?;
        let database = self.executor();
        let editor_view_instance_id = editor_source
            .as_ref()
            .map(|source| source.view_instance_id.clone());
        let delivery_mode = input.delivery_mode;
        let queued_client_message_id = client_message_id.clone();
        let queued_content = content.clone();
        let queued =
            a4n_db::transaction::run_in_transaction(database, "发送 Agent 消息", |tx| {
                Box::pin(async move {
                    let result = async {
                        let thread = agent::lock_thread(&tx, thread_id)
                            .await
                            .map_err(agent_db_error_to_app)?;
                        let runs = agent::active_runs_by_thread(&tx, thread_id)
                            .await
                            .map_err(agent_db_error_to_app)?;
                        let Some(active) = runs.into_iter().next() else {
                            return Ok(None);
                        };
                        if let Some(view_instance_id) = editor_view_instance_id.as_deref() {
                            let turn = agent::turn_by_id(&tx, active.turn_id)
                                .await
                                .map_err(agent_db_error_to_app)?
                                .ok_or_else(|| AppError::NotFound("Agent Turn 不存在".into()))?;
                            let original_source = agent::message_by_id(&tx, turn.input_message_id)
                                .await
                                .map_err(agent_db_error_to_app)?
                                .and_then(|message| message.payload_json)
                                .and_then(|payload| payload.get("editor_source").cloned());
                            let original = match original_source {
                                Some(value) => Some(
                                    parse_persisted_editor_source(database, value, &thread).await?,
                                ),
                                None => None,
                            };
                            if original.as_ref().is_none_or(|original| {
                                original.view_instance_id != view_instance_id
                            }) {
                                return Err(AppError::InvalidArgument(
                                    "当前任务来自其他视图；请等待或结束原任务后再从此视图发送"
                                        .into(),
                                ));
                            }
                        }
                        if active.status == "waiting"
                            && matches!(
                                active.waiting_reason.as_deref(),
                                Some("budget" | "reconcile")
                            )
                        {
                            return Err(AppError::InvalidState {
                                current: format!(
                                    "waiting({})",
                                    active.waiting_reason.as_deref().unwrap_or("unknown")
                                ),
                                operation: "发送 Agent 消息".into(),
                            });
                        }
                        let waiting_reason = active
                            .waiting_reason
                            .as_deref()
                            .filter(|reason| {
                                active.status == "waiting" && matches!(*reason, "user" | "backoff")
                            })
                            .map(str::to_string);
                        let message = agent::append_message(
                            &tx,
                            AgentMessageInput {
                                thread_id,
                                turn_id: Some(active.turn_id),
                                run_id: Some(active.id),
                                role: "user".into(),
                                sender_kind: "user".into(),
                                sender_thread_id: None,
                                client_message_id: Some(queued_client_message_id.clone()),
                                content: queued_content.clone(),
                                status: "complete".into(),
                                payload_json: Some(
                                    json!({ "delivery_mode": enum_value(delivery_mode) }),
                                ),
                            },
                        )
                        .await
                        .map_err(agent_db_error_to_app)?;
                        agent::enqueue_inbox(
                            &tx,
                            AgentInboxInput {
                                thread_id,
                                turn_id: Some(active.turn_id),
                                run_id: Some(active.id),
                                source_kind: "user".into(),
                                source_thread_id: None,
                                inbox_kind: enum_value(delivery_inbox_kind(delivery_mode)),
                                delivery_mode: enum_value(delivery_mode),
                                priority: delivery_priority(delivery_mode),
                                dedupe_key: format!("message:{queued_client_message_id}"),
                                content: queued_content,
                                payload_json: Some(json!({ "message_id": message.id.to_string() })),
                                available_at: None,
                            },
                        )
                        .await
                        .map_err(agent_db_error_to_app)?;
                        Ok(Some((active, message, waiting_reason)))
                    }
                    .await;
                    (tx, result)
                })
            })
            .await?;
        if let Some((active, message, waiting_reason)) = queued {
            let awakened = if let Some(reason) = waiting_reason {
                agent::wake_waiting_run_for_reason(self.executor(), active.id, &reason)
                    .await
                    .map_err(agent_db_error_to_app)?
            } else {
                None
            };
            self.append_user_event(
                user_id,
                UserEventFields {
                    thread_id,
                    turn_id: Some(active.turn_id),
                    run_id: Some(active.id),
                    message_id: Some(message.id),
                },
                "message_queued",
                "queued",
                "用户消息已加入 Agent inbox",
            )
            .await?;
            return accepted(
                thread_id,
                message.id,
                active.turn_id,
                active.id,
                awakened
                    .as_ref()
                    .map(|run| run.status.as_str())
                    .unwrap_or(active.status.as_str()),
                false,
            );
        }

        admit_new_run_runtime_snapshot(
            runtime_snapshot_required,
            require_runtime_snapshot_for_new_run,
        )?;

        let model_override_json = input
            .model_config_id
            .map(|id| json!({ "model_config_id": id }));
        validate_model_override_json(model_override_json.as_ref())?;
        let budget = input.budget.unwrap_or_default();
        validate_agent_budget(&budget)?;
        let budget_json = serde_json::to_value(budget)?;
        let (turn, run, message) = agent::create_turn_with_run(
            self.executor(),
            AgentTurnInput {
                thread_id,
                client_message_id: client_message_id.clone(),
                message: AgentMessageInput {
                    thread_id,
                    turn_id: None,
                    run_id: None,
                    role: "user".into(),
                    sender_kind: "user".into(),
                    sender_thread_id: None,
                    client_message_id: Some(client_message_id),
                    content: content.clone(),
                    status: "complete".into(),
                    payload_json: editor_source.map(|source| json!({"editor_source": source})),
                },
                execution_mode: enum_value(input.execution_mode),
                model_override_json,
                budget_json,
                runtime_snapshot_required,
                trigger: "user".into(),
                goal: Some(content),
            },
        )
        .await
        .map_err(agent_db_error_to_app)?;
        self.append_user_event(
            user_id,
            UserEventFields {
                thread_id,
                turn_id: Some(turn.id),
                run_id: Some(run.id),
                message_id: Some(message.id),
            },
            "run_queued",
            "queued",
            "Agent run 已排队",
        )
        .await?;
        accepted(thread_id, message.id, turn.id, run.id, &run.status, true)
    }

    /// Sends a user message to a child thread through an existing delegation.
    /// The parent ownership check prevents a user from addressing another
    /// user's child by guessing a delegation id.
    pub(crate) async fn send_message_to_child(
        &self,
        user_id: i64,
        input: SendAgentMessageToChildInput,
        runtime_snapshot_required: bool,
        require_runtime_snapshot_for_new_run: bool,
    ) -> Result<AgentMessageAccepted, AppError> {
        let delegation_id = parse_id(&input.delegation_id, "delegation_id")?;
        let delegation = agent::delegation_by_id(self.executor(), delegation_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| {
                AppError::NotFound(format!("Agent delegation 不存在: {delegation_id}"))
            })?;
        self.owned_thread(user_id, delegation.parent_thread_id)
            .await?;
        delegation
            .child_run_id
            .ok_or_else(|| AppError::InvalidState {
                current: format!("delegation {delegation_id} 尚未创建 child run"),
                operation: "发送 child Agent 消息".into(),
            })?;
        let child_thread_id = delegation.child_thread_id;
        let input = SendAgentMessageInput {
            thread_id: child_thread_id.to_string(),
            client_message_id: format!("user-child:{delegation_id}:{}", uuid::Uuid::new_v4()),
            content: input.content,
            delivery_mode: input.delivery_mode,
            ..SendAgentMessageInput::default()
        };
        let accepted = self
            .send_message(
                user_id,
                input,
                runtime_snapshot_required,
                require_runtime_snapshot_for_new_run,
            )
            .await?;
        Ok(accepted)
    }

    pub(crate) async fn control_child_agent(
        &self,
        user_id: i64,
        input: ControlChildAgentInput,
    ) -> Result<AgentRunView, AppError> {
        let delegation_id = parse_id(&input.delegation_id, "delegation_id")?;
        let delegation = agent::delegation_by_id(self.executor(), delegation_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| {
                AppError::NotFound(format!("Agent delegation 不存在: {delegation_id}"))
            })?;
        self.owned_thread(user_id, delegation.parent_thread_id)
            .await?;
        let child_run_id =
            agent::runs_by_thread(self.executor(), delegation.child_thread_id, SNAPSHOT_LIMIT)
                .await
                .map_err(agent_db_error_to_app)?
                .into_iter()
                .rev()
                .find(|row| agent::is_run_active(&row.status))
                .map(|row| row.id)
                .or(delegation.child_run_id)
                .ok_or_else(|| AppError::InvalidState {
                    current: format!("delegation {delegation_id} 尚未创建 child run"),
                    operation: "控制 child Agent".into(),
                })?;
        self.control_run(
            user_id,
            ControlAgentRunInput {
                thread_id: delegation.child_thread_id.to_string(),
                action: input.action,
                run_id: child_run_id.to_string(),
                budget: input.budget,
            },
        )
        .await
    }

    pub(crate) async fn spawn_agent(
        &self,
        user_id: i64,
        input: SpawnAgentInput,
    ) -> Result<AgentDelegationView, AppError> {
        validate_user_id(user_id)?;
        let parent_run_id = parse_id(&input.parent_run_id, "parent_run_id")?;
        let parent_run = agent::run_by_id(self.executor(), parent_run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| {
                AppError::NotFound(format!("Agent parent run 不存在: {parent_run_id}"))
            })?;
        let parent_thread = self.owned_thread(user_id, parent_run.thread_id).await?;
        if !agent::is_run_active(&parent_run.status) {
            return Err(AppError::InvalidState {
                current: parent_run.status,
                operation: "创建 Agent child delegation".into(),
            });
        }
        if matches!(
            parent_run.status.as_str(),
            "paused" | "pausing" | "cancelling"
        ) || (parent_run.status == "waiting"
            && parent_run.waiting_reason.as_deref() != Some("child"))
        {
            return Err(AppError::InvalidState {
                current: format!(
                    "{}({})",
                    parent_run.status,
                    parent_run.waiting_reason.as_deref().unwrap_or("none")
                ),
                operation: "创建 Agent child delegation".into(),
            });
        }
        let parent_turn = agent::turn_by_id(self.executor(), parent_run.turn_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| {
                AppError::NotFound(format!("Agent parent turn 不存在: {}", parent_run.turn_id))
            })?;
        validate_model_override_json(parent_turn.model_override_json.as_ref())?;

        let parent_budget: AgentBudget = serde_json::from_value(parent_run.budget_json.clone())
            .map_err(|error| AppError::InvalidState {
                current: format!("Agent run {} budget_json 无效: {error}", parent_run.id),
                operation: "创建 Agent child delegation".into(),
            })?;
        validate_agent_budget(&parent_budget)?;
        if !parent_run.usage_json.is_object() {
            return Err(AppError::InvalidState {
                current: format!("Agent run {} usage_json 不是对象", parent_run.id),
                operation: "创建 Agent child delegation".into(),
            });
        }
        let delegations = agent::delegations_by_thread(self.executor(), parent_thread.id)
            .await
            .map_err(agent_db_error_to_app)?;
        let delegation_snapshots = delegations
            .iter()
            .cloned()
            .map(delegation_snapshot)
            .collect::<Vec<AgentDelegationSnapshot>>();
        let delegation_depth = self
            .delegation_depth_for_run(parent_run.id, parent_run.parent_run_id)
            .await?;
        enforce_delegation_limits(
            &delegation_snapshots,
            parent_run.id,
            delegation_depth,
            &parent_budget,
        )?;

        let capability = required_capability(input.capability)?;
        let objective = required_text(input.task, "task")?;
        let mut scope = input.scope;
        // A user-operated child inherits the parent thread's resource fence by
        // default.  Explicit fields may narrow that fence, but cannot silently
        // turn a chapter/project run into an unrestricted global child.
        let parent_scope: AgentScope =
            from_json(parent_thread.scope_json.clone(), "agent_thread.scope_json")?;
        if !parent_scope.allowed_capabilities.is_empty()
            && !parent_scope
                .allowed_capabilities
                .iter()
                .any(|item| item == &capability)
        {
            return Err(AppError::Forbidden(format!(
                "Agent scope 不允许委派 capability: {capability}"
            )));
        }
        scope.resource_kind = scope.resource_kind.or(parent_scope.resource_kind.clone());
        scope.resource_id = scope.resource_id.or(parent_scope.resource_id.clone());
        scope.project_id = scope.project_id.or(parent_scope.project_id);
        scope.chapter_id = scope.chapter_id.or(parent_scope.chapter_id);
        apply_subagent_capability_policy(&capability, &mut scope)?;
        inherit_subagent_allowlists(&parent_scope, &mut scope)?;
        if parent_scope
            .project_id
            .is_some_and(|project_id| scope.project_id != Some(project_id))
            || parent_scope
                .chapter_id
                .is_some_and(|chapter_id| scope.chapter_id != Some(chapter_id))
            || (parent_scope.resource_kind.is_some()
                && (scope.resource_kind != parent_scope.resource_kind
                    || scope.resource_id != parent_scope.resource_id))
        {
            return Err(AppError::Forbidden(
                "child Agent scope 不能超出 parent Agent 资源范围".into(),
            ));
        }
        validate_scope(&scope)?;
        let mut bound_resource = input.bound_resource;
        if let Some(resource) = bound_resource.as_ref() {
            validate_resource_ref(resource)?;
            complete_scope_resource(&mut scope, &resource.kind, &resource.id)?;
        }
        // Domain capabilities keep their stable resource semantics. Generic
        // profiles inherit a chapter/project fence when one is present and
        // otherwise remain user-global.
        if bound_resource.is_none() {
            bound_resource = inferred_subagent_resource(&capability, &scope)?;
            if let Some(resource) = bound_resource.as_ref() {
                scope.resource_kind = Some(resource.kind.clone());
                scope.resource_id = Some(resource.id.clone());
            }
        }
        let (project_id, chapter_id, chapter_number) = self
            .verify_thread_scope(user_id, &scope, bound_resource.as_ref())
            .await?;
        scope.project_id = scope.project_id.or(project_id);
        scope.chapter_id = scope.chapter_id.or(chapter_id);
        scope.chapter_number = chapter_number;
        verify_director_source(self.executor(), &capability, &scope).await?;
        validate_subagent_resource(&capability, &scope, bound_resource.as_ref())?;

        apply_subagent_tool_policy(&capability, &mut scope)?;

        let canonical_binding_key = bound_resource
            .as_ref()
            .map(|resource| canonical_subagent_binding_key(&capability, scope.project_id, resource))
            .transpose()?
            .unwrap_or_else(|| format!("subagent:{capability}:global"));
        let child_thread = self
            .ensure_canonical_subagent_thread(
                user_id,
                EnsureCanonicalSubagentThreadInput {
                    canonical_binding_key,
                    capability: capability.clone(),
                    title: input.title,
                    scope: scope.clone(),
                    bound_resource: bound_resource.clone(),
                    default_model_config_id: parent_thread.default_model_config_id,
                },
            )
            .await?;
        let budget = child_budget_for_delegation(input.budget.unwrap_or_default(), &parent_budget)?;
        let delegation_key = format!(
            "user-spawn:{}:{}",
            parent_run_id,
            a4n_db::agent::sha256_hex(&serde_json::to_string(&(
                capability.as_str(),
                objective.as_str(),
                &scope
            ))?,)
        );
        let child_message_id = format!("delegation:{delegation_key}");
        let (delegation, _child_turn, _child_run, _child_message) = agent::create_user_delegation(
            self.executor(),
            AgentUserDelegationInput {
                delegation: AgentDelegationInput {
                    parent_thread_id: parent_thread.id,
                    parent_run_id,
                    parent_execution_id: None,
                    child_thread_id: child_thread.id,
                    child_turn_id: None,
                    child_run_id: None,
                    delegation_key,
                    capability,
                    objective: objective.clone(),
                    success_criteria_json: serde_json::to_value(input.success_criteria)?,
                    scope_json: serde_json::to_value(scope)?,
                    budget_json: serde_json::to_value(&budget)?,
                    status: "pending".into(),
                },
                child_turn: AgentTurnInput {
                    thread_id: child_thread.id,
                    client_message_id: child_message_id.clone(),
                    message: AgentMessageInput {
                        thread_id: child_thread.id,
                        turn_id: None,
                        run_id: None,
                        role: "user".into(),
                        sender_kind: "user".into(),
                        sender_thread_id: None,
                        client_message_id: Some(child_message_id),
                        content: objective.clone(),
                        status: "complete".into(),
                        payload_json: Some(json!({
                            "parent_run_id": parent_run_id.to_string(),
                            "user_spawn": true,
                        })),
                    },
                    execution_mode: parent_turn.execution_mode,
                    model_override_json: parent_turn.model_override_json,
                    budget_json: serde_json::to_value(&budget)?,
                    runtime_snapshot_required: input.runtime_config_snapshot.is_some(),
                    trigger: "delegation".into(),
                    goal: Some(objective.clone()),
                },
            },
        )
        .await
        .map_err(agent_db_error_to_app)?;
        self.append_user_event(
            user_id,
            UserEventFields {
                thread_id: parent_thread.id,
                turn_id: Some(parent_run.turn_id),
                run_id: Some(parent_run.id),
                message_id: None,
            },
            "delegation_started",
            "running",
            "用户已创建 Agent child delegation",
        )
        .await?;
        delegation_view(delegation)
    }

    pub(crate) async fn list_memories(
        &self,
        user_id: i64,
        scope_kind: Option<AgentMemoryScopeKind>,
        scope_id: Option<String>,
    ) -> Result<Vec<AgentMemoryView>, AppError> {
        validate_user_id(user_id)?;
        let kind = scope_kind.map(enum_value);
        let key = scope_id
            .as_deref()
            .map(|value| required_text(value.to_string(), "scope_id"))
            .transpose()?;
        let rows = agent::list_memory(self.executor(), user_id, kind.as_deref(), key.as_deref())
            .await
            .map_err(agent_db_error_to_app)?;
        rows.into_iter().map(memory_view).collect()
    }

    pub(crate) async fn create_memory(
        &self,
        user_id: i64,
        input: AgentMemoryInput,
    ) -> Result<AgentMemoryView, AppError> {
        validate_user_id(user_id)?;
        let content = required_text(input.content, "content")?;
        let memory_kind = required_text(input.memory_kind, "memory_kind")?;
        let scope_id = required_text(input.scope_id, "scope_id")?;
        let (project_id, thread_id) = match input.scope_kind {
            AgentMemoryScopeKind::User => {
                if scope_id != user_id.to_string() {
                    return Err(AppError::Forbidden("只能写入当前用户 memory".into()));
                }
                (None, None)
            }
            AgentMemoryScopeKind::Project => {
                let project_id = parse_id(&scope_id, "scope_id")?;
                a4n_db::collaboration::verify_project_permission(
                    self.executor(),
                    project_id,
                    user_id,
                    ProjectAccessRole::Viewer,
                )
                .await
                .map_err(collaboration_db_error_to_app)?;
                (Some(project_id), None)
            }
            AgentMemoryScopeKind::Thread => {
                let thread_id = parse_id(&scope_id, "scope_id")?;
                let thread = self.owned_thread(user_id, thread_id).await?;
                (thread.project_id, Some(thread_id))
            }
        };
        let existing = agent::list_memory(
            self.executor(),
            user_id,
            Some(&enum_value(input.scope_kind)),
            Some(&scope_id),
        )
        .await
        .map_err(agent_db_error_to_app)?;
        let revision = existing
            .iter()
            .filter(|row| row.memory_key == memory_kind)
            .map(|row| row.revision)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| AppError::InvalidArgument("memory revision 溢出".into()))?;
        let resource_refs = serde_json::to_value(input.resource_refs)?;
        let row = agent::insert_memory(
            self.executor(),
            a4n_domain::agent::AgentMemoryInput {
                owner_user_id: user_id,
                project_id,
                thread_id,
                scope_kind: enum_value(input.scope_kind),
                scope_key: scope_id,
                memory_key: memory_kind,
                revision,
                status: "active".into(),
                content,
                content_json: Some(json!({ "resource_refs": resource_refs })),
                source_message_id: None,
                source_execution_id: None,
                supersedes_memory_id: None,
            },
        )
        .await
        .map_err(agent_db_error_to_app)?;
        memory_view(row)
    }

    pub(crate) async fn retract_memory(
        &self,
        user_id: i64,
        input: RetractAgentMemoryInput,
    ) -> Result<AgentMemoryView, AppError> {
        validate_user_id(user_id)?;
        let memory_id = parse_id(&input.memory_id, "memory_id")?;
        let row = agent::retract_memory(self.executor(), user_id, memory_id)
            .await
            .map_err(agent_db_error_to_app)?;
        memory_view(row)
    }

    pub(crate) async fn control_run(
        &self,
        user_id: i64,
        input: ControlAgentRunInput,
    ) -> Result<AgentRunView, AppError> {
        let thread_id = parse_id(&input.thread_id, "thread_id")?;
        self.owned_thread(user_id, thread_id).await?;
        let runs = agent::runs_by_thread(self.executor(), thread_id, SNAPSHOT_LIMIT)
            .await
            .map_err(agent_db_error_to_app)?;
        let run_id = parse_id(&input.run_id, "run_id")?;
        let current = runs
            .iter()
            .find(|row| row.id == run_id)
            .ok_or_else(|| AppError::NotFound("可控制的 Agent run 不存在".into()))?;
        validate_control_transition(
            &current.status,
            current.waiting_reason.as_deref(),
            input.action,
            input.budget.is_some(),
        )?;
        let resume_budget = if let Some(budget) = input.budget.clone() {
            validate_agent_budget(&budget)?;
            let budget = if let Some(parent_run_id) = current.parent_run_id {
                let parent = agent::run_by_id(self.executor(), parent_run_id)
                    .await
                    .map_err(agent_db_error_to_app)?
                    .ok_or_else(|| {
                        AppError::NotFound(format!("Agent parent run 不存在: {parent_run_id}"))
                    })?;
                let parent_budget: AgentBudget =
                    from_json(parent.budget_json.clone(), "agent_run.parent.budget_json")?;
                validate_agent_budget(&parent_budget)?;
                let parent_usage = self.run_budget_snapshot(parent.id).await?.usage;
                child_budget_within_parent(budget, &parent_budget, &parent_usage)?
            } else {
                budget
            };
            Some(budget)
        } else {
            None
        };
        let updated = match input.action {
            AgentControlAction::Cancel => {
                let cancelled =
                    agent::cancel_run_tree(self.executor(), current.id, Some("用户取消 Agent run"))
                        .await
                        .map_err(agent_db_error_to_app)?;
                cancelled
                    .into_iter()
                    .find(|row| row.id == current.id)
                    .ok_or_else(|| {
                        AppError::NotFound(format!("Agent run 不存在: {}", current.id))
                    })?
            }
            AgentControlAction::Pause => {
                let paused = agent::pause_run_tree(self.executor(), current.id)
                    .await
                    .map_err(agent_db_error_to_app)?;
                paused
                    .into_iter()
                    .find(|row| row.id == current.id)
                    .ok_or_else(|| AppError::InvalidArgument("当前 Agent Run 不可暂停".into()))?
            }
            // Resume always starts a fresh claim cycle.  Never preserve an old
            // worker lease or mark the run running from a user request.
            AgentControlAction::Resume => agent::resume_run(
                self.executor(),
                user_id,
                thread_id,
                current.id,
                resume_budget.as_ref(),
            )
            .await
            .map_err(agent_db_error_to_app)?,
        };
        let status = updated.status.clone();
        self.append_user_event(
            user_id,
            UserEventFields {
                thread_id,
                turn_id: Some(updated.turn_id),
                run_id: Some(updated.id),
                message_id: None,
            },
            "run_controlled",
            &status,
            match input.action {
                AgentControlAction::Pause => "Agent run 已暂停",
                AgentControlAction::Resume => "Agent run 已恢复",
                AgentControlAction::Cancel => "Agent run 已取消",
            },
        )
        .await?;
        self.show_run(user_id, &updated.id.to_string()).await
    }

    pub(crate) async fn events(
        &self,
        user_id: i64,
        input: AgentEventsInput,
    ) -> Result<Vec<AgentEvent>, AppError> {
        let thread_id = parse_id(&input.thread_id, "thread_id")?;
        self.owned_thread(user_id, thread_id).await?;
        let after = i64::try_from(input.after_sequence.unwrap_or(0))
            .map_err(|_| AppError::InvalidArgument("after_sequence 超出范围".into()))?;
        let limit = bounded_limit(
            input.limit,
            AGENT_EVENTS_DEFAULT_LIMIT,
            AGENT_EVENTS_MAX_LIMIT,
            "limit",
        )? as u64;
        agent::events_after(self.executor(), thread_id, after, limit)
            .await
            .map_err(agent_db_error_to_app)?
            .into_iter()
            .map(event_view)
            .collect()
    }

    pub(crate) fn search_tools(
        &self,
        input: SearchAgentToolsInput,
    ) -> Result<AgentToolSearchResult, AppError> {
        let (tools, total) = self.search_tool_descriptors(input)?;
        Ok(AgentToolSearchResult {
            tools: tools.into_iter().map(Into::into).collect(),
            total,
        })
    }

    pub(super) fn search_tool_descriptors(
        &self,
        input: SearchAgentToolsInput,
    ) -> Result<(Vec<AgentToolDescriptor>, usize), AppError> {
        let query_terms = input
            .query
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|query| {
                query
                    .to_lowercase()
                    .split_whitespace()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .filter(|terms| !terms.is_empty());
        let capability = input
            .capability
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        // A registered domain capability is intentionally filtered to its
        // narrow catalog. Generic/user-defined capabilities are open to the
        // manifest's discoverable Host commands, so passing `general` (or a
        // custom label) must not silently hide the rest of the tool registry.
        let capability_tools = capability
            .filter(|value| a4n_api_contract::agent::capability_has_restricted_tool_catalog(value))
            .map(a4n_api_contract::agent::agent_tools_for_capability);
        let limit = bounded_limit(
            input.limit,
            AGENT_TOOLS_DEFAULT_LIMIT,
            AGENT_TOOLS_MAX_LIMIT,
            "limit",
        )? as usize;
        let manifest = a4n_api_contract::command_manifest::command_manifest(
            None,
            input.domain.as_deref(),
            input.effect.as_deref(),
        );
        let synthetic = a4n_api_contract::agent::synthetic_agent_tool_descriptors()
            .into_iter()
            .filter(|descriptor| {
                input
                    .domain
                    .as_deref()
                    .is_none_or(|domain| descriptor.domain == domain)
                    && input
                        .effect
                        .as_deref()
                        .is_none_or(|effect| descriptor.effect == effect)
            });
        let mut tools = synthetic
            .chain(manifest.commands)
            .filter_map(|descriptor| {
                let policy = descriptor.agent_policy?;
                if policy.exposure == AgentToolExposure::Hidden
                    || capability_tools
                        .is_some_and(|allowed| !allowed.contains(&descriptor.name.as_str()))
                    || query_terms.as_ref().is_some_and(|terms| {
                        !terms.iter().any(|term| {
                            descriptor.name.to_lowercase().contains(term)
                                || descriptor.summary.to_lowercase().contains(term)
                                || descriptor.domain.to_lowercase().contains(term)
                                || policy
                                    .tags
                                    .iter()
                                    .any(|tag| tag.to_lowercase().contains(term))
                        })
                    })
                {
                    return None;
                }
                Some(AgentToolDescriptor {
                    name: descriptor.name,
                    summary: descriptor.summary,
                    domain: descriptor.domain,
                    effect: descriptor.effect,
                    exposure: policy.exposure,
                    risk: policy.risk,
                    reversibility: policy.reversibility,
                    replay_policy: policy.replay_policy,
                    cost_class: policy.cost_class,
                    tags: policy.tags,
                    args_schema: descriptor.args_schema,
                    result_schema: descriptor.result_schema,
                })
            })
            .collect::<Vec<_>>();
        let mut seen = HashSet::with_capacity(tools.len());
        tools.retain(|tool| seen.insert(tool.name.clone()));
        let total = tools.len();
        tools.truncate(limit);
        Ok((tools, total))
    }

    pub(crate) async fn review_delegation(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::ReviewAgentDelegationInput,
        parent_run_id: Option<i64>,
    ) -> Result<AgentDelegationView, AppError> {
        use a4n_api_contract::agent::AgentDelegationReviewDecision;
        let id = parse_id(&input.delegation_id, "delegation_id")?;
        let row = agent::delegation_by_id(self.executor(), id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound("Agent delegation 不存在".into()))?;
        self.owned_thread(user_id, row.parent_thread_id).await?;
        if parent_run_id.is_some_and(|run_id| row.parent_run_id != run_id) {
            return Err(AppError::Forbidden(
                "Agent 只能审阅当前 Run 发起的委派".into(),
            ));
        }
        for resource in &input.reviewed_refs {
            validate_resource_ref(resource)?;
        }
        let row = agent::review_delegation(
            self.executor(),
            a4n_domain::agent::AgentDelegationReviewInput {
                user_id,
                delegation_id: id,
                expected_child_run_id: parse_id(
                    &input.expected_child_run_id,
                    "expected_child_run_id",
                )?,
                status: match input.decision {
                    AgentDelegationReviewDecision::Accept => "accepted",
                    AgentDelegationReviewDecision::RequestRevision => "revision_requested",
                }
                .into(),
                reason: required_text(input.reason, "reason")?,
                reviewed_refs: input
                    .reviewed_refs
                    .into_iter()
                    .map(serde_json::to_value)
                    .collect::<Result<_, _>>()?,
            },
        )
        .await
        .map_err(agent_db_error_to_app)?;
        delegation_view(row)
    }

    pub(crate) async fn resolve_approval(
        &self,
        user_id: i64,
        input: ResolveAgentApprovalInput,
    ) -> Result<AgentApprovalView, AppError> {
        validate_user_id(user_id)?;
        let approval_id = parse_id(&input.approval_id, "approval_id")?;
        let existing = agent::approval_by_id(self.executor(), approval_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent approval 不存在: {approval_id}")))?;
        self.owned_thread(user_id, existing.thread_id).await?;
        let approved = input.decision == AgentApprovalDecision::Approve;
        let row = agent::resolve_approval_for_run(
            self.executor(),
            approval_id,
            user_id,
            approved,
            optional_text(input.reason, "reason")?,
        )
        .await
        .map_err(agent_db_error_to_app)?;
        approval_view(row)
    }
}
