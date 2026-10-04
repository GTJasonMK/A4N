//! Agent thread、message 与 run 读取。

use super::*;

impl AgentApplication {
    pub(crate) async fn ensure_main_thread(
        &self,
        user_id: i64,
        input: EnsureMainAgentThreadInput,
    ) -> Result<AgentThreadView, AppError> {
        validate_user_id(user_id)?;
        if let Some(row) = agent::default_thread_by_owner(self.executor(), user_id)
            .await
            .map_err(agent_db_error_to_app)?
        {
            return thread_view(row);
        }

        let create = AgentThreadCreateInput {
            owner_user_id: user_id,
            thread_kind: "main".into(),
            profile_id: "main".into(),
            title: optional_text(input.title, "title")?,
            project_id: None,
            resource_type: None,
            resource_id: None,
            canonical_binding_key: None,
            scope_json: serde_json::to_value(AgentScope::default())?,
            default_model_config_id: None,
            is_default: true,
        };
        match agent::create_thread(self.executor(), create).await {
            Ok(row) => thread_view(row),
            Err(error @ AgentDbError::Database(_)) => {
                // Concurrent ensure requests may race on the partial unique index.
                match agent::default_thread_by_owner(self.executor(), user_id).await {
                    Ok(Some(row)) => thread_view(row),
                    Ok(None) | Err(_) => Err(agent_db_error_to_app(error)),
                }
            }
            Err(error) => Err(agent_db_error_to_app(error)),
        }
    }

    pub(crate) async fn create_thread(
        &self,
        user_id: i64,
        input: CreateAgentThreadInput,
    ) -> Result<AgentThreadView, AppError> {
        validate_user_id(user_id)?;
        validate_optional_positive_id(input.default_model_config_id, "default_model_config_id")?;
        let capability = normalized_capability(input.kind, input.capability)?;
        let bound_resource = input.bound_resource;
        if let Some(resource) = bound_resource.as_ref() {
            validate_resource_ref(resource)?;
        }
        validate_scope(&input.scope)?;
        let mut scope = input.scope;
        if let Some(resource) = &bound_resource {
            complete_scope_resource(&mut scope, &resource.kind, &resource.id)?;
        }
        let (project_id, chapter_id, chapter_number) = self
            .verify_thread_scope(user_id, &scope, bound_resource.as_ref())
            .await?;
        scope.project_id = scope.project_id.or(project_id);
        scope.chapter_id = scope.chapter_id.or(chapter_id);
        scope.chapter_number = chapter_number;
        verify_director_source(self.executor(), &capability, &scope).await?;
        if input.kind == AgentThreadKind::Subagent {
            validate_subagent_resource(&capability, &scope, bound_resource.as_ref())?;
            apply_subagent_capability_policy(&capability, &mut scope)?;
            apply_subagent_tool_policy(&capability, &mut scope)?;
        }
        let canonical_binding_key = bound_resource
            .as_ref()
            .map(|resource| canonical_subagent_binding_key(&capability, scope.project_id, resource))
            .transpose()?;
        let requested_scope = serde_json::to_value(&scope)?;
        if let Some(key) = canonical_binding_key.as_deref()
            && let Some(existing) =
                agent::thread_by_canonical_binding(self.executor(), user_id, key)
                    .await
                    .map_err(agent_db_error_to_app)?
        {
            return thread_view(
                self.reconcile_canonical_thread(
                    existing,
                    &capability,
                    &enum_value(input.kind),
                    &requested_scope,
                    "创建 Agent thread",
                )
                .await?,
            );
        }
        let row = agent::create_thread(
            self.executor(),
            AgentThreadCreateInput {
                owner_user_id: user_id,
                thread_kind: enum_value(input.kind),
                profile_id: capability.clone(),
                title: optional_text(input.title, "title")?,
                project_id,
                resource_type: bound_resource
                    .as_ref()
                    .map(|resource| resource.kind.clone()),
                resource_id: bound_resource.as_ref().map(|resource| resource.id.clone()),
                canonical_binding_key: canonical_binding_key.clone(),
                scope_json: requested_scope.clone(),
                default_model_config_id: input.default_model_config_id,
                is_default: false,
            },
        )
        .await;
        let row = match row {
            Ok(row) => row,
            Err(AgentDbError::Database(error)) if canonical_binding_key.is_some() => {
                let key = canonical_binding_key.as_deref().expect("checked above");
                let existing = agent::thread_by_canonical_binding(self.executor(), user_id, key)
                    .await
                    .map_err(agent_db_error_to_app)?
                    .ok_or(AppError::Database(anyhow::Error::from_boxed(error)))?;
                self.reconcile_canonical_thread(
                    existing,
                    &capability,
                    &enum_value(input.kind),
                    &requested_scope,
                    "创建 Agent thread",
                )
                .await?
            }
            Err(error) => return Err(agent_db_error_to_app(error)),
        };
        thread_view(row)
    }

    /// Creates a user-visible branch from an existing thread.  A fork is a
    /// new sub-agent thread; durable message/run history remains attached to
    /// the source thread and is intentionally not copied.
    pub(crate) async fn fork_thread(
        &self,
        user_id: i64,
        input: ForkAgentThreadInput,
    ) -> Result<AgentThreadView, AppError> {
        let source_id = parse_id(&input.thread_id, "thread_id")?;
        let source = self.owned_thread(user_id, source_id).await?;
        let scope = match input.scope {
            Some(scope) => scope,
            None => from_json(source.scope_json.clone(), "agent_thread.scope_json")?,
        };
        let bound_resource = input.bound_resource.or_else(|| {
            source
                .resource_type
                .clone()
                .zip(source.resource_id.clone())
                .map(|(kind, id)| AgentResourceRef {
                    kind,
                    id,
                    label: source.title.clone(),
                    revision: None,
                    hash: None,
                })
        });
        let title = input
            .title
            .or_else(|| source.title.clone().map(|title| format!("{title} (fork)")));
        self.create_thread(
            user_id,
            CreateAgentThreadInput {
                kind: AgentThreadKind::Subagent,
                capability: input.capability.or(Some(source.profile_id)),
                title,
                bound_resource,
                scope,
                default_model_config_id: source.default_model_config_id,
            },
        )
        .await
    }

    pub(crate) async fn update_thread(
        &self,
        user_id: i64,
        input: UpdateAgentThreadInput,
    ) -> Result<AgentThreadView, AppError> {
        let thread_id = parse_id(&input.thread_id, "thread_id")?;
        validate_user_id(user_id)?;
        let row = agent::thread_by_id(self.executor(), thread_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound("Agent thread 不存在".into()))?;
        if row.owner_user_id != user_id {
            return Err(AppError::Forbidden(
                "不能修改其他用户的 Agent thread".into(),
            ));
        }
        if let Some(expected) = input.expected_revision {
            let current = u64::try_from(row.revision).map_err(|_| AppError::InvalidState {
                current: format!("Agent thread {} revision 无效", row.id),
                operation: "更新 Agent thread".into(),
            })?;
            if expected != current {
                return Err(AppError::InvalidState {
                    current: format!(
                        "Agent thread revision 冲突: expected={expected}, current={current}"
                    ),
                    operation: "更新 Agent thread".into(),
                });
            }
        }
        let scope = if let Some(allowed_tools) = input.allowed_tools {
            if input.expected_revision.is_none() {
                return Err(AppError::InvalidArgument(
                    "修改工具权限必须提供 expected_revision".into(),
                ));
            }
            let mut scope: AgentScope =
                from_json(row.scope_json.clone(), "agent_thread.scope_json")?;
            scope.allowed_tools = allowed_tools;
            validate_scope(&scope)?;
            apply_subagent_tool_policy(&row.profile_id, &mut scope)?;
            Some(serde_json::to_value(scope)?)
        } else {
            None
        };
        let title = input
            .title
            .map(|title| optional_text(Some(title), "title"))
            .transpose()?;
        if let Some(model_config_id) = input.default_model_config_id {
            validate_optional_positive_id(Some(model_config_id), "default_model_config_id")?;
        }
        let updated =
            a4n_db::transaction::run_in_transaction(self.executor(), "更新 Agent thread", |tx| {
                Box::pin(async move {
                    let result = agent::update_thread_record(
                        &tx,
                        agent::AgentThreadUpdateRecordInput {
                            thread_id,
                            owner_user_id: user_id,
                            expected_revision: row.revision,
                            scope_json: scope,
                            title,
                            default_model_config_id: input.default_model_config_id,
                        },
                    )
                    .await
                    .map_err(agent_db_error_to_app);
                    (tx, result)
                })
            })
            .await?;
        thread_view(updated)
    }

    pub(crate) async fn archive_thread(
        &self,
        user_id: i64,
        input: ArchiveAgentThreadInput,
    ) -> Result<AgentThreadView, AppError> {
        let thread_id = parse_id(&input.thread_id, "thread_id")?;
        validate_user_id(user_id)?;
        let updated =
            a4n_db::transaction::run_in_transaction(self.executor(), "归档 Agent thread", |tx| {
                Box::pin(async move {
                    let result = agent::archive_thread_record(&tx, thread_id, user_id)
                        .await
                        .map_err(agent_db_error_to_app);
                    (tx, result)
                })
            })
            .await?;
        thread_view(updated)
    }

    /// Returns the stable domain thread used by a main Agent delegation.
    /// Creation is idempotent on the database unique binding key so concurrent
    /// model/tool workers converge on one child thread.
    pub(crate) async fn ensure_canonical_subagent_thread(
        &self,
        user_id: i64,
        input: EnsureCanonicalSubagentThreadInput,
    ) -> Result<agent::AgentThreadRecord, AppError> {
        validate_user_id(user_id)?;
        let key = required_text(input.canonical_binding_key, "canonical_binding_key")?;
        let capability = required_capability(input.capability)?;
        let mut scope = input.scope;
        validate_scope(&scope)?;
        apply_subagent_capability_policy(&capability, &mut scope)?;
        apply_subagent_tool_policy(&capability, &mut scope)?;
        let bound_resource = input.bound_resource;
        if let Some(resource) = bound_resource.as_ref() {
            validate_resource_ref(resource)?;
        }
        let (project_id, chapter_id, chapter_number) = self
            .verify_thread_scope(user_id, &scope, bound_resource.as_ref())
            .await?;
        scope.project_id = scope.project_id.or(project_id);
        scope.chapter_id = scope.chapter_id.or(chapter_id);
        scope.chapter_number = chapter_number;
        validate_subagent_resource(&capability, &scope, bound_resource.as_ref())?;
        let requested_scope = serde_json::to_value(&scope)?;
        if let Some(existing) = agent::thread_by_canonical_binding(self.executor(), user_id, &key)
            .await
            .map_err(agent_db_error_to_app)?
        {
            // Reusing a canonical thread must not change its explicit permissions.
            return self
                .reconcile_canonical_thread(
                    existing,
                    &capability,
                    "subagent",
                    &requested_scope,
                    "获取 canonical Agent thread",
                )
                .await;
        }
        let create = AgentThreadCreateInput {
            owner_user_id: user_id,
            thread_kind: "subagent".into(),
            profile_id: capability.clone(),
            title: optional_text(input.title, "title")?,
            project_id,
            resource_type: bound_resource
                .as_ref()
                .map(|resource| resource.kind.clone()),
            resource_id: bound_resource.as_ref().map(|resource| resource.id.clone()),
            canonical_binding_key: Some(key.clone()),
            scope_json: requested_scope.clone(),
            default_model_config_id: input.default_model_config_id,
            is_default: false,
        };
        match agent::create_thread(self.executor(), create).await {
            Ok(row) => Ok(row),
            Err(AgentDbError::Database(error)) => {
                let existing = agent::thread_by_canonical_binding(self.executor(), user_id, &key)
                    .await
                    .map_err(agent_db_error_to_app)?
                    .ok_or(AppError::Database(anyhow::Error::from_boxed(error)))?;
                self.reconcile_canonical_thread(
                    existing,
                    &capability,
                    "subagent",
                    &requested_scope,
                    "获取 canonical Agent thread",
                )
                .await
            }
            Err(error) => Err(agent_db_error_to_app(error)),
        }
    }

    pub(crate) async fn list_threads(
        &self,
        user_id: i64,
        input: ListAgentThreadsInput,
    ) -> Result<Vec<AgentThreadView>, AppError> {
        validate_user_id(user_id)?;
        let capability = input
            .capability
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let limit = bounded_limit(
            input.limit,
            AGENT_THREADS_DEFAULT_LIMIT,
            AGENT_THREADS_MAX_LIMIT,
            "limit",
        )? as usize;
        agent::list_threads_by_owner(self.executor(), user_id)
            .await
            .map_err(agent_db_error_to_app)?
            .into_iter()
            .filter(|row| input.include_archived || row.archived_at.is_none())
            .filter(|row| capability.is_none_or(|capability| row.profile_id == capability))
            .take(limit)
            .map(thread_view)
            .collect()
    }

    pub(crate) async fn show_thread(
        &self,
        user_id: i64,
        thread_id: &str,
    ) -> Result<AgentThreadHistorySnapshot, AppError> {
        let thread_id = parse_id(thread_id, "thread_id")?;
        self.owned_thread(user_id, thread_id).await?;
        let history = agent::thread_history(self.executor(), user_id, thread_id, SNAPSHOT_LIMIT)
            .await
            .map_err(agent_db_error_to_app)?;

        Ok(AgentThreadHistorySnapshot {
            protocol_version: AGENT_API_PROTOCOL_VERSION,
            thread: thread_view(history.thread)?,
            messages: history
                .messages
                .into_iter()
                .map(message_view)
                .collect::<Result<_, _>>()?,
            turns: history
                .turns
                .into_iter()
                .map(turn_view)
                .collect::<Result<_, _>>()?,
            runs: history
                .runs
                .into_iter()
                .map(run_view)
                .collect::<Result<_, _>>()?,
            executions: history
                .executions
                .into_iter()
                .map(execution_view)
                .collect::<Result<_, _>>()?,
            delegations: history
                .delegations
                .into_iter()
                .map(delegation_view)
                .collect::<Result<_, _>>()?,
            approvals: history
                .approvals
                .into_iter()
                .map(approval_view)
                .collect::<Result<_, _>>()?,
            inbox: history
                .inbox
                .into_iter()
                .map(inbox_view)
                .collect::<Result<_, _>>()?,
            memories: history
                .memories
                .into_iter()
                .map(memory_view)
                .collect::<Result<_, _>>()?,
            events: history
                .events
                .into_iter()
                .map(event_view)
                .collect::<Result<_, _>>()?,
        })
    }

    pub(crate) async fn list_messages(
        &self,
        user_id: i64,
        input: ListAgentMessagesInput,
    ) -> Result<Vec<AgentMessageView>, AppError> {
        let thread_id = parse_id(&input.thread_id, "thread_id")?;
        self.owned_thread(user_id, thread_id).await?;
        let limit = bounded_limit(
            input.limit,
            AGENT_EVENTS_DEFAULT_LIMIT,
            SNAPSHOT_LIMIT as u32,
            "limit",
        )? as usize;
        let before_id = input
            .before
            .as_deref()
            .map(|before| parse_id(before, "before"))
            .transpose()?;
        let rows = agent::messages_by_thread(self.executor(), thread_id, before_id, limit as u64)
            .await
            .map_err(agent_db_error_to_app)?;
        rows.into_iter()
            .rev()
            .take(limit)
            .map(message_view)
            .collect()
    }

    pub(crate) async fn show_run(
        &self,
        user_id: i64,
        run_id: &str,
    ) -> Result<AgentRunView, AppError> {
        let run_id = parse_id(run_id, "run_id")?;
        let mut run = agent::run_by_id(self.executor(), run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent run 不存在: {run_id}")))?;
        self.owned_thread(user_id, run.thread_id).await?;
        run.usage_json = self.run_budget_snapshot(run.id).await?.usage;
        run_view(run)
    }
}
