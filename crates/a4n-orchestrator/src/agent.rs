//! Durable control plane for the shared main/sub-agent runtime.
//!
//! This application owns user authorization, command discovery and API
//! projections. Model/provider execution stays outside this module so the
//! runtime can drive queued runs without introducing a second workflow owner.

use a4n_api_contract::agent::{
    AGENT_API_PROTOCOL_VERSION, AGENT_EVENTS_DEFAULT_LIMIT, AGENT_EVENTS_MAX_LIMIT,
    AGENT_THREADS_DEFAULT_LIMIT, AGENT_THREADS_MAX_LIMIT, AGENT_TOOLS_DEFAULT_LIMIT,
    AGENT_TOOLS_MAX_LIMIT, AgentApprovalDecision, AgentApprovalStatus, AgentApprovalView,
    AgentBudget, AgentControlAction, AgentDelegationStatus, AgentDelegationView, AgentDeliveryMode,
    AgentEvent, AgentEventsInput, AgentExecutionKind, AgentExecutionStatus, AgentExecutionView,
    AgentInboxItemView, AgentInboxKind, AgentInboxStatus, AgentMemoryInput, AgentMemoryScopeKind,
    AgentMemoryStatus, AgentMemoryView, AgentMessageAccepted, AgentMessageRole, AgentMessageStatus,
    AgentMessageView, AgentResourceRef, AgentRunStatus, AgentRunView, AgentScope, AgentThreadKind,
    AgentThreadSnapshot as AgentThreadHistorySnapshot, AgentThreadStatus, AgentThreadView,
    AgentToolDescriptor, AgentToolExposure, AgentToolReplayPolicy, AgentToolSearchResult,
    AgentTurnStatus, AgentTurnTrigger, AgentTurnView, AgentWaitingReason, ArchiveAgentThreadInput,
    ControlAgentRunInput, ControlChildAgentInput, CreateAgentThreadInput,
    EnsureMainAgentThreadInput, ForkAgentThreadInput, GENERAL_AGENT_CAPABILITY,
    ListAgentMessagesInput, ListAgentThreadsInput, MAIN_AGENT_CAPABILITY,
    ResolveAgentApprovalInput, RetractAgentMemoryInput, SearchAgentToolsInput,
    SendAgentMessageInput, SendAgentMessageToChildInput, SpawnAgentInput, UpdateAgentThreadInput,
};
use a4n_db::DbHandle;
use a4n_db::agent::{self, AgentDbError, AgentUserDelegationInput};
use a4n_db::collaboration::CollaborationDbError;
use a4n_db_models::models::agent::{
    approval, delegation, event, execution, inbox, memory, message, run, thread, turn,
};
use a4n_domain::agent::{
    AgentDelegationInput, AgentEventInput, AgentExecutionInput, AgentInboxInput, AgentMessageInput,
    AgentThreadCreateInput, AgentTurnInput,
};
use a4n_error::AppError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::collections::HashSet;

use crate::collaboration::ProjectAccessRole;

const SNAPSHOT_LIMIT: u64 = 1_000;
mod approval_expiry;
mod budget;
mod editor_claim;
mod editor_response;
mod editor_wait;
mod external_task_wait;
mod generation;
mod generation_cancel;
mod lease_recovery;
mod worker_activation;
mod worker_approval;
mod worker_claim;
mod worker_completion;
mod worker_context;
mod worker_delegation;
mod worker_execution;
mod worker_failure;
mod worker_history;
mod worker_inbox;
mod worker_lease;
mod worker_message;
mod worker_read;
mod worker_recovery;
mod worker_usage;
mod worker_wait;
mod workflow_binding;
mod workflow_cancel;
pub use approval_expiry::AgentApprovalExpiryApplicationPort;
pub use budget::{AgentWorkerBudgetApplicationPort, AgentWorkerBudgetSnapshot};
pub use editor_claim::AgentEditorClaimApplicationPort;
pub use editor_response::AgentEditorResponseApplicationPort;
pub use editor_wait::AgentEditorWaitApplicationPort;
pub use external_task_wait::{AgentExternalTaskWaitApplicationPort, AgentExternalTaskWaitSnapshot};
pub use generation::model_request_snapshot;
pub use generation::restore_model_request_snapshot;
pub use generation_cancel::{AgentGenerationCancelApplicationPort, AgentGenerationCancelSnapshot};
pub use lease_recovery::AgentRunLeaseRecoveryApplicationPort;
pub use worker_activation::AgentWorkerActivationApplicationPort;
pub use worker_approval::AgentWorkerApprovalApplicationPort;
pub use worker_claim::AgentWorkerClaimApplicationPort;
pub use worker_completion::AgentWorkerCompletionApplicationPort;
pub use worker_context::AgentWorkerContextApplicationPort;
pub use worker_delegation::AgentWorkerDelegationApplicationPort;
pub use worker_execution::AgentWorkerExecutionApplicationPort;
pub use worker_failure::AgentWorkerFailureApplicationPort;
pub use worker_history::AgentWorkerHistoryApplicationPort;
pub use worker_inbox::AgentWorkerInboxApplicationPort;
pub use worker_lease::AgentWorkerLeaseApplicationPort;
pub use worker_message::AgentWorkerMessageApplicationPort;
pub use worker_read::AgentWorkerReadApplicationPort;
pub use worker_recovery::AgentWorkerRecoveryApplicationPort;
pub use worker_usage::AgentWorkerUsageApplicationPort;
pub use worker_wait::AgentWorkerWaitApplicationPort;
pub use workflow_binding::AgentWorkflowBindingApplicationPort;
pub use workflow_cancel::{AgentWorkflowCancelApplicationPort, AgentWorkflowCancelSnapshot};

#[derive(Clone)]
pub struct AgentApplication {
    database: DbHandle,
}

impl AgentApplication {
    /// Lifecycle shutdown uses the same durable run-tree cancellation owner as
    /// the user-facing control path, but does not impersonate a user.
    pub(crate) async fn cancel_run_for_package_shutdown(
        &self,
        run_id: i64,
    ) -> Result<(), AppError> {
        agent::cancel_run_tree(
            self.connection(),
            run_id,
            Some("功能包停用，取消 Agent run"),
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }

    pub(crate) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    fn connection(&self) -> &sea_orm::DatabaseConnection {
        self.database.connection()
    }

    fn reconcile_canonical_thread(
        &self,
        existing: thread::Model,
        capability: &str,
        expected_kind: &str,
        requested_scope: &Value,
        operation: &str,
    ) -> Result<thread::Model, AppError> {
        if existing.thread_kind != expected_kind || existing.profile_id != capability {
            return Err(AppError::InvalidState {
                current: format!("canonical Agent thread {} 配置不一致", existing.id),
                operation: operation.into(),
            });
        }
        let existing_scope: AgentScope =
            from_json(existing.scope_json.clone(), "agent_thread.scope_json")?;
        let mut requested: AgentScope = from_json(requested_scope.clone(), "requested_scope")?;
        inherit_subagent_allowlists(&existing_scope, &mut requested)?;
        if existing_scope == requested {
            return Ok(existing);
        }
        Err(AppError::InvalidArgument(
            "canonical Agent thread 已有不同 scope；请通过 update_agent_thread 显式修改，不能在重用时覆盖权限".into(),
        ))
    }

    pub(crate) async fn ensure_main_thread(
        &self,
        user_id: i64,
        input: EnsureMainAgentThreadInput,
    ) -> Result<AgentThreadView, AppError> {
        validate_user_id(user_id)?;
        if let Some(row) = agent::default_thread_by_owner(self.connection(), user_id)
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
        match agent::create_thread(self.connection(), create).await {
            Ok(row) => thread_view(row),
            Err(error @ AgentDbError::Database(_)) => {
                // Concurrent ensure requests may race on the partial unique index.
                match agent::default_thread_by_owner(self.connection(), user_id).await {
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
        let (project_id, chapter_id) = self
            .verify_thread_scope(user_id, &scope, bound_resource.as_ref())
            .await?;
        scope.project_id = scope.project_id.or(project_id);
        scope.chapter_id = scope.chapter_id.or(chapter_id);
        if input.kind == AgentThreadKind::Subagent {
            validate_subagent_resource(&capability, &scope, bound_resource.as_ref())?;
            apply_subagent_capability_policy(&capability, &mut scope)?;
            apply_subagent_tool_policy(&capability, &mut scope)?;
        }
        let canonical_binding_key = bound_resource
            .as_ref()
            .map(|resource| canonical_subagent_binding_key(&capability, resource))
            .transpose()?;
        let requested_scope = serde_json::to_value(&scope)?;
        if let Some(key) = canonical_binding_key.as_deref()
            && let Some(existing) =
                agent::thread_by_canonical_binding(self.connection(), user_id, key)
                    .await
                    .map_err(agent_db_error_to_app)?
        {
            return thread_view(self.reconcile_canonical_thread(
                existing,
                &capability,
                &enum_value(input.kind),
                &requested_scope,
                "创建 Agent thread",
            )?);
        }
        let row = agent::create_thread(
            self.connection(),
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
                let existing = agent::thread_by_canonical_binding(self.connection(), user_id, key)
                    .await
                    .map_err(agent_db_error_to_app)?
                    .ok_or(AppError::Database(error))?;
                self.reconcile_canonical_thread(
                    existing,
                    &capability,
                    &enum_value(input.kind),
                    &requested_scope,
                    "创建 Agent thread",
                )?
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
        let tx = self.connection().begin().await?;
        let row = thread::Entity::find_by_id(thread_id)
            .lock_exclusive()
            .one(&tx)
            .await?
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
            if run::Entity::find()
                .filter(run::Column::ThreadId.eq(thread_id))
                .filter(run::Column::IsActive.eq(true))
                .one(&tx)
                .await?
                .is_some()
            {
                return Err(AppError::InvalidArgument(
                    "Agent thread 存在活动 Run，不能修改工具权限".into(),
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
        let current_revision = row.revision;
        let mut active: thread::ActiveModel = row.into();
        if let Some(scope) = scope {
            active.scope_json = Set(scope);
        }
        if let Some(title) = input.title {
            active.title = Set(optional_text(Some(title), "title")?);
        }
        if let Some(model_config_id) = input.default_model_config_id {
            validate_optional_positive_id(Some(model_config_id), "default_model_config_id")?;
            active.default_model_config_id = Set(Some(model_config_id));
        }
        active.revision = Set(current_revision
            .checked_add(1)
            .ok_or_else(|| AppError::InvalidArgument("Agent thread revision 溢出".into()))?);
        active.updated_at = Set(chrono::Utc::now().naive_utc());
        let updated = active.update(&tx).await.map_err(AppError::Database)?;
        tx.commit().await?;
        thread_view(updated)
    }

    pub(crate) async fn archive_thread(
        &self,
        user_id: i64,
        input: ArchiveAgentThreadInput,
    ) -> Result<AgentThreadView, AppError> {
        let thread_id = parse_id(&input.thread_id, "thread_id")?;
        let row = self.owned_thread(user_id, thread_id).await?;
        if row.is_default {
            return Err(AppError::InvalidState {
                current: "default".into(),
                operation: "归档 Agent thread".into(),
            });
        }
        let runs = agent::runs_by_thread(self.connection(), thread_id, SNAPSHOT_LIMIT)
            .await
            .map_err(agent_db_error_to_app)?;
        if runs.iter().any(|run| run.is_active) {
            return Err(AppError::InvalidState {
                current: "running".into(),
                operation: "归档 Agent thread".into(),
            });
        }
        let current_revision = row.revision;
        let mut active: thread::ActiveModel = row.into();
        active.status = Set("archived".into());
        active.archived_at = Set(Some(chrono::Utc::now().naive_utc()));
        active.revision = Set(current_revision
            .checked_add(1)
            .ok_or_else(|| AppError::InvalidArgument("Agent thread revision 溢出".into()))?);
        active.updated_at = Set(chrono::Utc::now().naive_utc());
        let updated = active
            .update(self.connection())
            .await
            .map_err(AppError::Database)?;
        thread_view(updated)
    }

    /// Returns the stable domain thread used by a main Agent delegation.
    /// Creation is idempotent on the database unique binding key so concurrent
    /// model/tool workers converge on one child thread.
    pub(crate) async fn ensure_canonical_subagent_thread(
        &self,
        user_id: i64,
        input: EnsureCanonicalSubagentThreadInput,
    ) -> Result<thread::Model, AppError> {
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
        let (project_id, chapter_id) = self
            .verify_thread_scope(user_id, &scope, bound_resource.as_ref())
            .await?;
        scope.project_id = scope.project_id.or(project_id);
        scope.chapter_id = scope.chapter_id.or(chapter_id);
        validate_subagent_resource(&capability, &scope, bound_resource.as_ref())?;
        let requested_scope = serde_json::to_value(&scope)?;
        if let Some(existing) = agent::thread_by_canonical_binding(self.connection(), user_id, &key)
            .await
            .map_err(agent_db_error_to_app)?
        {
            // Reusing a canonical thread must not change its explicit permissions.
            return self.reconcile_canonical_thread(
                existing,
                &capability,
                "subagent",
                &requested_scope,
                "获取 canonical Agent thread",
            );
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
        match agent::create_thread(self.connection(), create).await {
            Ok(row) => Ok(row),
            Err(AgentDbError::Database(error)) => {
                let existing = agent::thread_by_canonical_binding(self.connection(), user_id, &key)
                    .await
                    .map_err(agent_db_error_to_app)?
                    .ok_or(AppError::Database(error))?;
                self.reconcile_canonical_thread(
                    existing,
                    &capability,
                    "subagent",
                    &requested_scope,
                    "获取 canonical Agent thread",
                )
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
        agent::list_threads_by_owner(self.connection(), user_id)
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
        let thread_row = self.owned_thread(user_id, thread_id).await?;
        let messages = agent::messages_by_thread(self.connection(), thread_id, SNAPSHOT_LIMIT)
            .await
            .map_err(agent_db_error_to_app)?;
        let turns = agent::turns_by_thread(self.connection(), thread_id, SNAPSHOT_LIMIT)
            .await
            .map_err(agent_db_error_to_app)?;
        let mut runs = agent::runs_by_thread(self.connection(), thread_id, SNAPSHOT_LIMIT)
            .await
            .map_err(agent_db_error_to_app)?;
        let executions =
            agent::executions_by_runs(self.connection(), runs.iter().map(|row| row.id).collect())
                .await
                .map_err(agent_db_error_to_app)?;
        let delegations = agent::delegations_by_thread(self.connection(), thread_id)
            .await
            .map_err(agent_db_error_to_app)?;
        let approvals = agent::approvals_by_thread(self.connection(), thread_id)
            .await
            .map_err(agent_db_error_to_app)?;
        let inbox = agent::inbox_by_thread(self.connection(), thread_id)
            .await
            .map_err(agent_db_error_to_app)?;
        let memories = agent::list_memory(
            self.connection(),
            user_id,
            Some("thread"),
            Some(&thread_id.to_string()),
        )
        .await
        .map_err(agent_db_error_to_app)?;
        let events = agent::events_after(self.connection(), thread_id, 0, SNAPSHOT_LIMIT)
            .await
            .map_err(agent_db_error_to_app)?;
        for run in &mut runs {
            run.usage_json = self.run_budget_snapshot(run.id).await?.usage;
        }

        Ok(AgentThreadHistorySnapshot {
            protocol_version: AGENT_API_PROTOCOL_VERSION,
            thread: thread_view(thread_row)?,
            messages: messages
                .into_iter()
                .map(message_view)
                .collect::<Result<_, _>>()?,
            turns: turns.into_iter().map(turn_view).collect::<Result<_, _>>()?,
            runs: runs.into_iter().map(run_view).collect::<Result<_, _>>()?,
            executions: executions
                .into_iter()
                .map(execution_view)
                .collect::<Result<_, _>>()?,
            delegations: delegations
                .into_iter()
                .map(delegation_view)
                .collect::<Result<_, _>>()?,
            approvals: approvals
                .into_iter()
                .map(approval_view)
                .collect::<Result<_, _>>()?,
            inbox: inbox
                .into_iter()
                .map(inbox_view)
                .collect::<Result<_, _>>()?,
            memories: memories
                .into_iter()
                .map(memory_view)
                .collect::<Result<_, _>>()?,
            events: events
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
        let mut rows = agent::messages_by_thread(self.connection(), thread_id, SNAPSHOT_LIMIT)
            .await
            .map_err(agent_db_error_to_app)?;
        if let Some(before) = input.before {
            let before_id = parse_id(&before, "before")?;
            rows.retain(|row| row.id < before_id);
        }
        rows.into_iter()
            .rev()
            .take(limit)
            .map(message_view)
            .collect()
    }

    /// Loads a message referenced by an inbox item for the durable worker.
    /// The thread check keeps a malformed or cross-thread payload from being
    /// injected into the model context.
    pub(crate) async fn load_message_for_worker(
        &self,
        thread_id: i64,
        message_id: i64,
    ) -> Result<AgentMessageSnapshot, AppError> {
        let row = agent::message_by_id(self.connection(), message_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent message 不存在: {message_id}")))?;
        if row.thread_id != thread_id {
            return Err(AppError::InvalidState {
                current: format!(
                    "Agent inbox message {} 不属于 thread {}",
                    message_id, thread_id
                ),
                operation: "加载 Agent inbox message".into(),
            });
        }
        Ok(row.into())
    }

    pub(crate) async fn show_run(
        &self,
        user_id: i64,
        run_id: &str,
    ) -> Result<AgentRunView, AppError> {
        let run_id = parse_id(run_id, "run_id")?;
        let mut run = agent::run_by_id(self.connection(), run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent run 不存在: {run_id}")))?;
        self.owned_thread(user_id, run.thread_id).await?;
        run.usage_json = self.run_budget_snapshot(run.id).await?.usage;
        run_view(run)
    }

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
        input: SendAgentMessageInput,
        runtime_snapshot_required: bool,
        require_runtime_snapshot_for_new_run: bool,
    ) -> Result<AgentMessageAccepted, AppError> {
        let thread_id = parse_id(&input.thread_id, "thread_id")?;
        let thread = self.owned_thread(user_id, thread_id).await?;
        if let Some(source) = &input.editor_source {
            validate_editor_source(source, &thread)?;
        }
        let client_message_id = required_text(input.client_message_id, "client_message_id")?;
        let content = required_text(input.content, "content")?;
        validate_optional_positive_id(input.model_config_id, "model_config_id")?;
        let tx = self.connection().begin().await?;
        thread::Entity::find_by_id(thread_id)
            .lock_exclusive()
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Agent Thread 不存在".into()))?;
        let runs = agent::runs_by_thread(&tx, thread_id, SNAPSHOT_LIMIT)
            .await
            .map_err(agent_db_error_to_app)?;
        if let Some(active) = runs.iter().rev().find(|row| row.is_active) {
            if let Some(source) = &input.editor_source {
                let turn = turn::Entity::find_by_id(active.turn_id)
                    .one(self.connection())
                    .await?
                    .ok_or_else(|| AppError::NotFound("Agent Turn 不存在".into()))?;
                let original = agent::message_by_id(self.connection(), turn.input_message_id)
                    .await
                    .map_err(agent_db_error_to_app)?
                    .and_then(|message| message.payload_json)
                    .and_then(|payload| payload.get("editor_source").cloned())
                    .map(|value| {
                        from_json::<a4n_api_contract::agent::AgentEditorSource>(
                            value,
                            "editor_source",
                        )
                    })
                    .transpose()?;
                if original
                    .as_ref()
                    .is_none_or(|original| original.view_instance_id != source.view_instance_id)
                {
                    return Err(AppError::InvalidArgument(
                        "当前任务来自其他视图；请等待或结束原任务后再从此视图发送".into(),
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
                    client_message_id: Some(client_message_id.clone()),
                    content: content.clone(),
                    status: "complete".into(),
                    payload_json: Some(json!({ "delivery_mode": enum_value(input.delivery_mode) })),
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
                    inbox_kind: enum_value(delivery_inbox_kind(input.delivery_mode)),
                    delivery_mode: enum_value(input.delivery_mode),
                    priority: delivery_priority(input.delivery_mode),
                    dedupe_key: format!("message:{client_message_id}"),
                    content,
                    payload_json: Some(json!({ "message_id": message.id.to_string() })),
                    available_at: None,
                },
            )
            .await
            .map_err(agent_db_error_to_app)?;
            tx.commit().await?;
            let awakened = if let Some(reason) = waiting_reason {
                agent::wake_waiting_run_for_reason(self.connection(), active.id, &reason)
                    .await
                    .map_err(agent_db_error_to_app)?
            } else {
                None
            };
            self.append_user_event(
                user_id,
                thread_id,
                Some(active.turn_id),
                Some(active.id),
                Some(message.id),
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

        tx.commit().await?;

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
            self.connection(),
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
                    payload_json: input
                        .editor_source
                        .map(|source| json!({"editor_source": source})),
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
            thread_id,
            Some(turn.id),
            Some(run.id),
            Some(message.id),
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
        let delegation = agent::delegation_by_id(self.connection(), delegation_id)
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
        let delegation = agent::delegation_by_id(self.connection(), delegation_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| {
                AppError::NotFound(format!("Agent delegation 不存在: {delegation_id}"))
            })?;
        self.owned_thread(user_id, delegation.parent_thread_id)
            .await?;
        let child_run_id = agent::runs_by_thread(
            self.connection(),
            delegation.child_thread_id,
            SNAPSHOT_LIMIT,
        )
        .await
        .map_err(agent_db_error_to_app)?
        .into_iter()
        .rev()
        .find(|row| row.is_active)
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
        let parent_run = agent::run_by_id(self.connection(), parent_run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| {
                AppError::NotFound(format!("Agent parent run 不存在: {parent_run_id}"))
            })?;
        let parent_thread = self.owned_thread(user_id, parent_run.thread_id).await?;
        if !parent_run.is_active
            || matches!(
                parent_run.status.as_str(),
                "completed" | "failed" | "cancelled" | "interrupted" | "recovery_required"
            )
        {
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
        let parent_turn = turn::Entity::find_by_id(parent_run.turn_id)
            .one(self.connection())
            .await
            .map_err(AppError::Database)?
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
        let delegations = agent::delegations_by_thread(self.connection(), parent_thread.id)
            .await
            .map_err(agent_db_error_to_app)?;
        let delegation_snapshots = delegations
            .iter()
            .cloned()
            .map(Into::into)
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
            if (scope.resource_kind.is_some() || scope.resource_id.is_some())
                && (scope.resource_kind.as_deref() != Some(resource.kind.as_str())
                    || scope.resource_id.as_deref() != Some(resource.id.as_str()))
            {
                return Err(AppError::InvalidArgument(
                    "Agent scope 与 bound_resource 不一致".into(),
                ));
            }
            scope.resource_kind = Some(resource.kind.clone());
            scope.resource_id = Some(resource.id.clone());
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
        let (project_id, chapter_id) = self
            .verify_thread_scope(user_id, &scope, bound_resource.as_ref())
            .await?;
        scope.project_id = scope.project_id.or(project_id);
        scope.chapter_id = scope.chapter_id.or(chapter_id);
        validate_subagent_resource(&capability, &scope, bound_resource.as_ref())?;

        apply_subagent_tool_policy(&capability, &mut scope)?;

        let canonical_binding_key = bound_resource
            .as_ref()
            .map(|resource| canonical_subagent_binding_key(&capability, resource))
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
        let budget = child_budget_within_parent(
            input.budget.unwrap_or_default(),
            &parent_budget,
            &self.run_budget_snapshot(parent_run.id).await?.usage,
        )?;
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
            self.connection(),
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
            parent_thread.id,
            Some(parent_run.turn_id),
            Some(parent_run.id),
            None,
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
        let rows = agent::list_memory(self.connection(), user_id, kind.as_deref(), key.as_deref())
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
                    self.connection(),
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
            self.connection(),
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
            self.connection(),
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
        let row = memory::Entity::find_by_id(memory_id)
            .one(self.connection())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("Agent memory 不存在: {memory_id}")))?;
        if row.owner_user_id != user_id {
            return Err(AppError::Forbidden("无权修改此 Agent memory".into()));
        }
        let mut active: memory::ActiveModel = row.into();
        active.status = Set("retracted".into());
        active.retracted_at = Set(Some(chrono::Utc::now().naive_utc()));
        active.updated_at = Set(chrono::Utc::now().naive_utc());
        let row = active
            .update(self.connection())
            .await
            .map_err(AppError::Database)?;
        memory_view(row)
    }

    pub(crate) async fn control_run(
        &self,
        user_id: i64,
        input: ControlAgentRunInput,
    ) -> Result<AgentRunView, AppError> {
        let thread_id = parse_id(&input.thread_id, "thread_id")?;
        self.owned_thread(user_id, thread_id).await?;
        let runs = agent::runs_by_thread(self.connection(), thread_id, SNAPSHOT_LIMIT)
            .await
            .map_err(agent_db_error_to_app)?;
        let run_id = parse_id(&input.run_id, "run_id")?;
        let current = runs
            .iter()
            .find(|row| row.id == run_id)
            .ok_or_else(|| AppError::NotFound("可控制的 Agent run 不存在".into()))?;
        validate_control_transition(&current.status, input.action)?;
        let (updated, status) = match input.action {
            AgentControlAction::Cancel => {
                let cancelled = agent::cancel_run_tree(
                    self.connection(),
                    current.id,
                    Some("用户取消 Agent run"),
                )
                .await
                .map_err(agent_db_error_to_app)?;
                let updated = cancelled
                    .into_iter()
                    .find(|row| row.id == current.id)
                    .ok_or_else(|| {
                        AppError::NotFound(format!("Agent run 不存在: {}", current.id))
                    })?;
                (updated, "cancelled")
            }
            AgentControlAction::Pause => (
                agent::update_run_state(
                    self.connection(),
                    current.id,
                    None,
                    "paused",
                    Some("user"),
                    None,
                )
                .await
                .map_err(agent_db_error_to_app)?,
                "paused",
            ),
            // Resume always starts a fresh claim cycle.  Never preserve an old
            // worker lease or mark the run running from a user request.
            AgentControlAction::Resume => (
                agent::update_run_state(self.connection(), current.id, None, "queued", None, None)
                    .await
                    .map_err(agent_db_error_to_app)?,
                "queued",
            ),
        };
        self.append_user_event(
            user_id,
            thread_id,
            Some(updated.turn_id),
            Some(updated.id),
            None,
            "run_controlled",
            status,
            match input.action {
                AgentControlAction::Pause => "Agent run 已暂停",
                AgentControlAction::Resume => "Agent run 已恢复",
                AgentControlAction::Cancel => "Agent run 已取消",
            },
        )
        .await?;
        run_view(updated)
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
        agent::events_after(self.connection(), thread_id, after, limit)
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

    fn search_tool_descriptors(
        &self,
        input: SearchAgentToolsInput,
    ) -> Result<(Vec<AgentToolDescriptor>, usize), AppError> {
        let query = input
            .query
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_lowercase);
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
                    || query.as_ref().is_some_and(|query| {
                        !descriptor.name.to_lowercase().contains(query)
                            && !descriptor.summary.to_lowercase().contains(query)
                            && !descriptor.domain.to_lowercase().contains(query)
                            && !policy
                                .tags
                                .iter()
                                .any(|tag| tag.to_lowercase().contains(query))
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
        let row = delegation::Entity::find_by_id(id)
            .one(self.connection())
            .await?
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
            self.connection(),
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
        let existing = agent::approval_by_id(self.connection(), approval_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent approval 不存在: {approval_id}")))?;
        self.owned_thread(user_id, existing.thread_id).await?;
        let approved = input.decision == AgentApprovalDecision::Approve;
        let row = agent::resolve_approval_for_run(
            self.connection(),
            approval_id,
            user_id,
            approved,
            optional_text(input.reason, "reason")?,
        )
        .await
        .map_err(agent_db_error_to_app)?;
        approval_view(row)
    }

    async fn owned_thread(&self, user_id: i64, thread_id: i64) -> Result<thread::Model, AppError> {
        validate_user_id(user_id)?;
        let row = agent::thread_by_id(self.connection(), thread_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent thread 不存在: {thread_id}")))?;
        if row.owner_user_id != user_id {
            return Err(AppError::Forbidden("无权访问此 Agent thread".into()));
        }
        Ok(row)
    }

    async fn verify_thread_scope(
        &self,
        user_id: i64,
        scope: &AgentScope,
        bound_resource: Option<&AgentResourceRef>,
    ) -> Result<(Option<i64>, Option<i64>), AppError> {
        let mut project_id = scope.project_id;
        let mut chapter_id = scope.chapter_id;
        if let (Some(kind), Some(id)) = (&scope.resource_kind, &scope.resource_id) {
            collect_scoped_resource(kind, id, &mut project_id, &mut chapter_id)?;
        }
        if let Some(resource) = bound_resource {
            collect_scoped_resource(
                &resource.kind,
                &resource.id,
                &mut project_id,
                &mut chapter_id,
            )?;
        }
        if let Some(chapter_id) = chapter_id {
            let chapter = a4n_db::project::chapter_by_id(self.connection(), chapter_id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))?;
            merge_scope_id(&mut project_id, chapter.project_id, "project_id")?;
        }
        if (scope.resource_kind.is_some() || bound_resource.is_some()) && project_id.is_none() {
            return Err(AppError::InvalidArgument(
                "非项目/章节资源绑定必须在 scope.project_id 中声明所属项目".into(),
            ));
        }
        if let Some(project_id) = project_id {
            a4n_db::collaboration::verify_project_permission(
                self.connection(),
                project_id,
                user_id,
                ProjectAccessRole::Viewer,
            )
            .await
            .map_err(collaboration_db_error_to_app)?;
        }
        Ok((project_id, chapter_id))
    }

    /// Validates chapter identifiers found inside a dynamically discovered
    /// Host command against a project-bound Agent scope. Argument names alone
    /// cannot prove this relationship, so the durable chapter row is the
    /// single source of truth at the runtime boundary.
    pub(crate) async fn verify_chapters_in_project(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_ids: &[i64],
    ) -> Result<(), AppError> {
        validate_user_id(user_id)?;
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("projectId 无效".into()));
        }
        a4n_db::collaboration::verify_project_permission(
            self.connection(),
            project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await
        .map_err(collaboration_db_error_to_app)?;
        let mut seen = HashSet::new();
        for chapter_id in chapter_ids {
            if !seen.insert(*chapter_id) {
                continue;
            }
            if *chapter_id <= 0 {
                return Err(AppError::InvalidArgument("chapterId 无效".into()));
            }
            let chapter = a4n_db::project::chapter_by_id(self.connection(), *chapter_id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))?;
            if chapter.project_id != project_id {
                return Err(AppError::Forbidden(format!(
                    "章节 {chapter_id} 不属于 Agent scope 项目 {project_id}"
                )));
            }
        }
        Ok(())
    }

    async fn append_user_event(
        &self,
        user_id: i64,
        thread_id: i64,
        turn_id: Option<i64>,
        run_id: Option<i64>,
        message_id: Option<i64>,
        kind: &str,
        status: &str,
        public_text: &str,
    ) -> Result<(), AppError> {
        agent::append_event(
            self.connection(),
            AgentEventInput {
                thread_id,
                turn_id,
                run_id,
                execution_id: None,
                delegation_id: None,
                message_id,
                actor_kind: "user".into(),
                actor_id: Some(user_id.to_string()),
                kind: kind.into(),
                status: status.into(),
                public_text: public_text.into(),
                payload_json: None,
            },
        )
        .await
        .map_err(agent_db_error_to_app)?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::agent::AgentApplicationPort for AgentApplication {
    async fn ensure_main_thread(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::EnsureMainAgentThreadInput,
    ) -> Result<a4n_api_contract::agent::AgentThreadView, AppError> {
        AgentApplication::ensure_main_thread(self, user_id, input).await
    }

    async fn create_thread(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::CreateAgentThreadInput,
    ) -> Result<a4n_api_contract::agent::AgentThreadView, AppError> {
        AgentApplication::create_thread(self, user_id, input).await
    }

    async fn fork_thread(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::ForkAgentThreadInput,
    ) -> Result<a4n_api_contract::agent::AgentThreadView, AppError> {
        AgentApplication::fork_thread(self, user_id, input).await
    }

    async fn update_thread(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::UpdateAgentThreadInput,
    ) -> Result<a4n_api_contract::agent::AgentThreadView, AppError> {
        AgentApplication::update_thread(self, user_id, input).await
    }

    async fn archive_thread(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::ArchiveAgentThreadInput,
    ) -> Result<a4n_api_contract::agent::AgentThreadView, AppError> {
        AgentApplication::archive_thread(self, user_id, input).await
    }

    async fn list_threads(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::ListAgentThreadsInput,
    ) -> Result<Vec<a4n_api_contract::agent::AgentThreadView>, AppError> {
        AgentApplication::list_threads(self, user_id, input).await
    }

    async fn show_thread(
        &self,
        user_id: i64,
        thread_id: &str,
    ) -> Result<a4n_api_contract::agent::AgentThreadSnapshot, AppError> {
        AgentApplication::show_thread(self, user_id, thread_id).await
    }

    async fn list_messages(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::ListAgentMessagesInput,
    ) -> Result<Vec<a4n_api_contract::agent::AgentMessageView>, AppError> {
        AgentApplication::list_messages(self, user_id, input).await
    }

    async fn show_run(
        &self,
        user_id: i64,
        run_id: &str,
    ) -> Result<a4n_api_contract::agent::AgentRunView, AppError> {
        AgentApplication::show_run(self, user_id, run_id).await
    }

    async fn send_message(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::SendAgentMessageInput,
        runtime_snapshot_required: bool,
        require_runtime_snapshot_for_new_run: bool,
    ) -> Result<a4n_api_contract::agent::AgentMessageAccepted, AppError> {
        AgentApplication::send_message(
            self,
            user_id,
            input,
            runtime_snapshot_required,
            require_runtime_snapshot_for_new_run,
        )
        .await
    }

    async fn control_run(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::ControlAgentRunInput,
    ) -> Result<a4n_api_contract::agent::AgentRunView, AppError> {
        AgentApplication::control_run(self, user_id, input).await
    }

    async fn send_message_to_child(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::SendAgentMessageToChildInput,
        runtime_snapshot_required: bool,
        require_runtime_snapshot_for_new_run: bool,
    ) -> Result<a4n_api_contract::agent::AgentMessageAccepted, AppError> {
        AgentApplication::send_message_to_child(
            self,
            user_id,
            input,
            runtime_snapshot_required,
            require_runtime_snapshot_for_new_run,
        )
        .await
    }

    async fn control_child_agent(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::ControlChildAgentInput,
    ) -> Result<a4n_api_contract::agent::AgentRunView, AppError> {
        AgentApplication::control_child_agent(self, user_id, input).await
    }

    async fn spawn_agent(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::SpawnAgentInput,
    ) -> Result<a4n_api_contract::agent::AgentDelegationView, AppError> {
        AgentApplication::spawn_agent(self, user_id, input).await
    }

    async fn events(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::AgentEventsInput,
    ) -> Result<Vec<a4n_api_contract::agent::AgentEvent>, AppError> {
        AgentApplication::events(self, user_id, input).await
    }

    fn search_tools(
        &self,
        input: a4n_api_contract::agent::SearchAgentToolsInput,
    ) -> Result<a4n_api_contract::agent::AgentToolSearchResult, AppError> {
        AgentApplication::search_tools(self, input)
    }

    fn describe_tool(
        &self,
        input: a4n_api_contract::agent::DescribeAgentToolInput,
    ) -> Result<a4n_api_contract::agent::AgentToolDescriptor, AppError> {
        AgentApplication::describe_tool(self, input)
    }

    async fn list_memories(
        &self,
        user_id: i64,
        scope_kind: Option<a4n_api_contract::agent::AgentMemoryScopeKind>,
        scope_id: Option<String>,
    ) -> Result<Vec<a4n_api_contract::agent::AgentMemoryView>, AppError> {
        AgentApplication::list_memories(self, user_id, scope_kind, scope_id).await
    }

    async fn create_memory(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::AgentMemoryInput,
    ) -> Result<a4n_api_contract::agent::AgentMemoryView, AppError> {
        AgentApplication::create_memory(self, user_id, input).await
    }

    async fn retract_memory(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::RetractAgentMemoryInput,
    ) -> Result<a4n_api_contract::agent::AgentMemoryView, AppError> {
        AgentApplication::retract_memory(self, user_id, input).await
    }

    async fn review_delegation(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::ReviewAgentDelegationInput,
        parent_run_id: Option<i64>,
    ) -> Result<a4n_api_contract::agent::AgentDelegationView, AppError> {
        AgentApplication::review_delegation(self, user_id, input, parent_run_id).await
    }

    async fn resolve_approval(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::ResolveAgentApprovalInput,
    ) -> Result<a4n_api_contract::agent::AgentApprovalView, AppError> {
        AgentApplication::resolve_approval(self, user_id, input).await
    }

    async fn verify_chapters_in_project(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_ids: &[i64],
    ) -> Result<(), AppError> {
        AgentApplication::verify_chapters_in_project(self, user_id, project_id, chapter_ids).await
    }
}

// -------------------------------------------------------------------------
// Durable worker façade
// -------------------------------------------------------------------------
//
// The runtime worker must not reach through the orchestrator to SeaORM.  The
// small façade below is the only worker-facing persistence surface: every
// mutating operation is fenced by the Run lease where the database primitive
// supports it, and all public output is persisted as an event/message before
// the worker can report progress.

/// Stable worker-facing projection of a durable Agent run.
///
/// Runtime actions consume this projection instead of the SeaORM row. The
/// database owner remains free to change persistence columns without leaking
/// them into the worker crate.
#[derive(Debug, Clone)]
pub struct AgentRunSnapshot {
    pub id: i64,
    pub thread_id: i64,
    pub turn_id: i64,
    pub parent_run_id: Option<i64>,
    pub status: String,
    pub budget_json: Value,
    pub usage_json: Value,
    pub lease_token: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub started_at: Option<chrono::NaiveDateTime>,
}

/// Durable facts carried between Agent runs.  It is deliberately a data-only
/// contract: values here are context, never instructions or executable state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentContextSummary {
    pub current_goal: String,
    pub constraints: Vec<String>,
    pub decisions: Vec<String>,
    pub facts_resources: Vec<String>,
    pub completed: Vec<String>,
    pub pending: Vec<String>,
    pub tool_state: Vec<String>,
    pub covered_through_sequence: i64,
}

const MAX_CONTEXT_SUMMARY_BYTES: usize = 256 * 1024;

impl AgentContextSummary {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.covered_through_sequence < 0 || self.current_goal.chars().count() > 4_000 {
            return Err(AppError::InvalidState {
                current: "Agent context_summary 的目标或覆盖序列无效".into(),
                operation: "校验 Agent context_summary".into(),
            });
        }
        for (name, values) in [
            ("constraints", &self.constraints),
            ("decisions", &self.decisions),
            ("facts_resources", &self.facts_resources),
            ("completed", &self.completed),
            ("pending", &self.pending),
            ("tool_state", &self.tool_state),
        ] {
            if values.len() > 64 || values.iter().any(|value| value.chars().count() > 4_000) {
                return Err(AppError::InvalidState {
                    current: format!("Agent context_summary.{name} 超出大小限制"),
                    operation: "校验 Agent context_summary".into(),
                });
            }
        }
        if serde_json::to_vec(self)?.len() > MAX_CONTEXT_SUMMARY_BYTES {
            return Err(AppError::InvalidState {
                current: format!(
                    "Agent context_summary 超过 {} KiB 限制",
                    MAX_CONTEXT_SUMMARY_BYTES / 1024
                ),
                operation: "校验 Agent context_summary".into(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct AgentThreadSnapshot {
    pub id: i64,
    pub owner_user_id: i64,
    pub thread_kind: String,
    pub profile_id: String,
    pub title: Option<String>,
    pub project_id: Option<i64>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub scope_json: Value,
    pub default_model_config_id: Option<i64>,
    pub status: String,
    pub revision: i64,
    pub context_summary: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct AgentTurnSnapshot {
    pub id: i64,
    pub thread_id: i64,
    pub input_message_id: i64,
    pub execution_mode: String,
    pub goal: Option<String>,
    pub model_override_json: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct AgentMessageSnapshot {
    pub id: i64,
    pub thread_id: i64,
    pub turn_id: Option<i64>,
    pub run_id: Option<i64>,
    pub role: String,
    pub content: String,
    pub payload_json: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct AgentExecutionSnapshot {
    pub id: i64,
    pub run_id: i64,
    pub execution_key: String,
    pub execution_kind: String,
    pub status: String,
    pub tool_name: Option<String>,
    pub effect: Option<String>,
    pub replay_policy: Option<String>,
    pub arguments_json: Option<Value>,
    pub resource_preconditions_json: Option<Value>,
    pub result_json: Option<Value>,
    pub result_summary: Option<String>,
    pub result_refs_json: Option<Value>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AgentDelegationSnapshot {
    pub id: i64,
    pub parent_run_id: i64,
    pub parent_execution_id: Option<i64>,
    pub child_thread_id: i64,
    pub child_turn_id: Option<i64>,
    pub child_run_id: Option<i64>,
    pub delegation_key: String,
    pub capability: String,
    pub objective: String,
    pub success_criteria_json: Value,
    pub scope_json: Value,
    pub budget_json: Value,
    pub status: String,
    pub result_refs_json: Option<Value>,
    pub result_summary: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AgentInboxSnapshot {
    pub id: i64,
    pub thread_id: i64,
    pub turn_id: Option<i64>,
    pub run_id: Option<i64>,
    pub source_kind: String,
    pub source_thread_id: Option<i64>,
    pub inbox_kind: String,
    pub delivery_mode: String,
    pub content: String,
    pub payload_json: Option<Value>,
    pub status: String,
    pub claim_token: Option<String>,
}

impl From<run::Model> for AgentRunSnapshot {
    fn from(row: run::Model) -> Self {
        Self {
            id: row.id,
            thread_id: row.thread_id,
            turn_id: row.turn_id,
            parent_run_id: row.parent_run_id,
            status: row.status,
            budget_json: row.budget_json,
            usage_json: row.usage_json,
            lease_token: row.lease_token,
            created_at: row.created_at,
            started_at: row.started_at,
        }
    }
}

fn thread_snapshot(row: thread::Model) -> Result<AgentThreadSnapshot, AppError> {
    let context_summary = parse_agent_context_summary(row.context_summary.as_deref())?;
    Ok(AgentThreadSnapshot {
        id: row.id,
        owner_user_id: row.owner_user_id,
        thread_kind: row.thread_kind,
        profile_id: row.profile_id,
        title: row.title,
        project_id: row.project_id,
        resource_type: row.resource_type,
        resource_id: row.resource_id,
        scope_json: row.scope_json,
        default_model_config_id: row.default_model_config_id,
        status: row.status,
        revision: row.revision,
        context_summary,
    })
}

pub fn parse_agent_context_summary(raw: Option<&str>) -> Result<Option<Value>, AppError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let summary: AgentContextSummary =
        serde_json::from_str(raw).map_err(|error| AppError::InvalidState {
            current: format!("Agent thread context_summary 无效: {error}"),
            operation: "读取 Agent context_summary".into(),
        })?;
    summary.validate()?;
    serde_json::to_value(summary)
        .map(Some)
        .map_err(AppError::from)
}

impl From<turn::Model> for AgentTurnSnapshot {
    fn from(row: turn::Model) -> Self {
        Self {
            id: row.id,
            thread_id: row.thread_id,
            input_message_id: row.input_message_id,
            execution_mode: row.execution_mode,
            goal: row.goal,
            model_override_json: row.model_override_json,
        }
    }
}

impl From<message::Model> for AgentMessageSnapshot {
    fn from(row: message::Model) -> Self {
        Self {
            id: row.id,
            thread_id: row.thread_id,
            turn_id: row.turn_id,
            run_id: row.run_id,
            role: row.role,
            content: row.content,
            payload_json: row.payload_json,
        }
    }
}

impl From<execution::Model> for AgentExecutionSnapshot {
    fn from(row: execution::Model) -> Self {
        Self {
            id: row.id,
            run_id: row.run_id,
            execution_key: row.execution_key,
            execution_kind: row.execution_kind,
            status: row.status,
            tool_name: row.tool_name,
            effect: row.effect,
            replay_policy: row.replay_policy,
            arguments_json: row.arguments_json,
            resource_preconditions_json: row.resource_preconditions_json,
            result_json: row.result_json,
            result_summary: row.result_summary,
            result_refs_json: row.result_refs_json,
            error_message: row.error_message,
        }
    }
}

impl From<delegation::Model> for AgentDelegationSnapshot {
    fn from(row: delegation::Model) -> Self {
        Self {
            id: row.id,
            parent_run_id: row.parent_run_id,
            parent_execution_id: row.parent_execution_id,
            child_thread_id: row.child_thread_id,
            child_turn_id: row.child_turn_id,
            child_run_id: row.child_run_id,
            delegation_key: row.delegation_key,
            capability: row.capability,
            objective: row.objective,
            success_criteria_json: row.success_criteria_json,
            scope_json: row.scope_json,
            budget_json: row.budget_json,
            status: row.status,
            result_refs_json: row.result_refs_json,
            result_summary: row.result_summary,
            error_message: row.error_message,
        }
    }
}

impl From<inbox::Model> for AgentInboxSnapshot {
    fn from(row: inbox::Model) -> Self {
        Self {
            id: row.id,
            thread_id: row.thread_id,
            turn_id: row.turn_id,
            run_id: row.run_id,
            source_kind: row.source_kind,
            source_thread_id: row.source_thread_id,
            inbox_kind: row.inbox_kind,
            delivery_mode: row.delivery_mode,
            content: row.content,
            payload_json: row.payload_json,
            status: row.status,
            claim_token: row.claim_token,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentWorkerRun {
    pub run: AgentRunSnapshot,
    pub thread: AgentThreadSnapshot,
    pub lease_token: String,
}

#[derive(Debug, Clone)]
pub struct AgentRunContext {
    pub run: AgentRunSnapshot,
    pub thread: AgentThreadSnapshot,
    pub turn: AgentTurnSnapshot,
    pub messages: Vec<AgentMessageSnapshot>,
    pub delegations: Vec<AgentDelegationSnapshot>,
    pub executions: Vec<AgentExecutionSnapshot>,
    pub editor_source: Option<a4n_api_contract::agent::AgentEditorSource>,
    pub owner_user_id: i64,
    pub scope: AgentScope,
    pub profile_id: String,
    pub default_model_config_id: Option<i64>,
    pub execution_mode: String,
    pub goal: Option<String>,
    pub budget: Value,
    pub usage: Value,
    pub runtime_snapshot_required: bool,
    pub delegation_depth: u8,
}

/// Validates the durable Agent budget contract before a Run is admitted or
/// resumed. Runtime-specific accounting (for example provider cost) remains
/// the worker's responsibility, but malformed/non-positive limits must never
/// enter the run tree.
pub fn validate_agent_budget(budget: &AgentBudget) -> Result<(), AppError> {
    let positive = [
        budget.max_duration_seconds,
        budget.max_tokens,
        budget.max_cost_micros,
    ]
    .into_iter()
    .flatten()
    .all(|value| value > 0)
        && [
            budget.max_steps,
            budget.max_model_calls,
            budget.max_tool_calls,
        ]
        .into_iter()
        .flatten()
        .all(|value| value > 0)
        && [budget.max_parallel_tools, budget.max_children]
            .into_iter()
            .flatten()
            .all(|value| value > 0)
        && budget.max_delegation_depth.is_none_or(|value| value > 0);
    if !positive {
        return Err(AppError::InvalidArgument(
            "Agent budget 上限必须为正数".into(),
        ));
    }
    if budget
        .max_duration_seconds
        .is_some_and(|value| value > i64::MAX as u64)
    {
        return Err(AppError::InvalidArgument(
            "Agent max_duration_seconds 超出时间实现上限".into(),
        ));
    }
    if budget.max_cost_micros.is_some() {
        return Err(AppError::InvalidArgument(
            "当前 provider 没有可靠的 Agent cost usage，暂不支持 max_cost_micros".into(),
        ));
    }
    Ok(())
}

/// Applies the parent Run's child-count and lineage-depth limits. The caller
/// supplies the already loaded delegation snapshot so the same policy is used
/// by model-issued and user-issued child creation paths.
pub fn enforce_delegation_limits(
    delegations: &[AgentDelegationSnapshot],
    run_id: i64,
    depth: u8,
    budget: &AgentBudget,
) -> Result<(), AppError> {
    if budget.max_children.is_some_and(|limit| {
        delegations
            .iter()
            .filter(|row| row.parent_run_id == run_id)
            .count()
            >= usize::from(limit)
    }) {
        return Err(AppError::InvalidState {
            current: format!("Agent run {run_id} 已达到 max_children"),
            operation: "创建 child Agent".into(),
        });
    }
    if budget
        .max_delegation_depth
        .is_some_and(|limit| depth >= limit)
    {
        return Err(AppError::InvalidState {
            current: format!("Agent run {run_id} 已达到 max_delegation_depth"),
            operation: "创建 child Agent".into(),
        });
    }
    Ok(())
}

/// Caps a child budget by the parent's remaining usage and nesting budget.
/// This is intentionally kept beside the budget validator so all child
/// admission paths share one contract.
pub fn child_budget_within_parent(
    mut child: AgentBudget,
    parent: &AgentBudget,
    usage: &Value,
) -> Result<AgentBudget, AppError> {
    validate_agent_budget(&child)?;
    let steps_used = budget_usage_u32(usage, "steps_used")?;
    let model_calls_used = budget_usage_u32(usage, "model_calls_used")?;
    let tool_calls_used = budget_usage_u32(usage, "tool_calls_used")?;
    let tokens_used = budget_usage_u64(usage, "tokens_used")?;
    ensure_budget_remaining_u32(parent.max_steps, steps_used, "max_steps")?;
    ensure_budget_remaining_u32(parent.max_model_calls, model_calls_used, "max_model_calls")?;
    ensure_budget_remaining_u32(parent.max_tool_calls, tool_calls_used, "max_tool_calls")?;
    ensure_budget_remaining_u64(parent.max_tokens, tokens_used, "max_tokens")?;
    cap_budget_u32(&mut child.max_steps, parent.max_steps, steps_used);
    cap_budget_u32(
        &mut child.max_model_calls,
        parent.max_model_calls,
        model_calls_used,
    );
    cap_budget_u32(
        &mut child.max_tool_calls,
        parent.max_tool_calls,
        tool_calls_used,
    );
    cap_budget_u64(&mut child.max_tokens, parent.max_tokens, tokens_used);
    cap_budget_u64(
        &mut child.max_duration_seconds,
        parent.max_duration_seconds,
        0,
    );
    cap_budget_u16(&mut child.max_parallel_tools, parent.max_parallel_tools);
    cap_budget_u16(&mut child.max_children, parent.max_children);
    cap_budget_u8(
        &mut child.max_delegation_depth,
        parent
            .max_delegation_depth
            .map(|limit| limit.saturating_sub(1).max(1)),
    );
    Ok(child)
}

fn ensure_budget_remaining_u32(limit: Option<u32>, used: u32, field: &str) -> Result<(), AppError> {
    if limit.is_some_and(|limit| used >= limit) {
        return Err(AppError::InvalidState {
            current: format!("parent Agent budget {field} 已耗尽"),
            operation: "创建 child Agent".into(),
        });
    }
    Ok(())
}

fn ensure_budget_remaining_u64(limit: Option<u64>, used: u64, field: &str) -> Result<(), AppError> {
    if limit.is_some_and(|limit| used >= limit) {
        return Err(AppError::InvalidState {
            current: format!("parent Agent budget {field} 已耗尽"),
            operation: "创建 child Agent".into(),
        });
    }
    Ok(())
}

fn cap_budget_u32(child: &mut Option<u32>, parent: Option<u32>, used: u32) {
    if let Some(limit) = parent {
        let remaining = limit.saturating_sub(used).max(1);
        *child = Some(child.unwrap_or(remaining).min(remaining));
    }
}

fn cap_budget_u64(child: &mut Option<u64>, parent: Option<u64>, used: u64) {
    if let Some(limit) = parent {
        let remaining = limit.saturating_sub(used).max(1);
        *child = Some(child.unwrap_or(remaining).min(remaining));
    }
}

fn cap_budget_u16(child: &mut Option<u16>, parent: Option<u16>) {
    if let Some(limit) = parent {
        *child = Some(child.unwrap_or(limit).min(limit));
    }
}

fn cap_budget_u8(child: &mut Option<u8>, parent: Option<u8>) {
    if let Some(limit) = parent {
        *child = Some(child.unwrap_or(limit).min(limit));
    }
}

fn budget_usage_u32(value: &Value, key: &str) -> Result<u32, AppError> {
    u32::try_from(budget_usage_u64(value, key)?).map_err(|_| AppError::InvalidState {
        current: format!("Agent usage.{key} 超出 u32"),
        operation: "读取 Agent run budget usage".into(),
    })
}

fn budget_usage_u64(value: &Value, key: &str) -> Result<u64, AppError> {
    match value.get(key) {
        None => Ok(0),
        Some(value) => value.as_u64().ok_or_else(|| AppError::InvalidState {
            current: format!("Agent usage.{key} 不是非负整数"),
            operation: "读取 Agent run budget usage".into(),
        }),
    }
}

#[derive(Debug, Clone)]
pub struct WorkerExecutionInput {
    pub run_id: i64,
    pub lease_token: String,
    pub execution_key: String,
    pub execution_kind: String,
    pub tool_name: Option<String>,
    pub effect: Option<String>,
    pub replay_policy: Option<String>,
    pub arguments_json: Option<Value>,
    pub resource_preconditions_json: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct WorkerExecutionUpdate {
    pub run_id: i64,
    pub lease_token: String,
    pub execution_id: i64,
    pub status: String,
    pub result_json: Option<Value>,
    pub result_summary: Option<String>,
    pub result_refs_json: Option<Value>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkerUsage {
    pub run_id: i64,
    pub lease_token: String,
    pub usage_json: Value,
}

#[derive(Debug, Clone)]
pub struct WorkerDelegationInput {
    pub capability: String,
    pub objective: String,
    pub delegation_key: String,
    pub child_thread_id: i64,
    pub scope: AgentScope,
    pub success_criteria: Vec<String>,
    pub budget_json: Value,
    pub parent_execution_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct WorkerDelegationResult {
    pub delegation: AgentDelegationSnapshot,
    pub child_turn: AgentTurnSnapshot,
    pub child_run: AgentRunSnapshot,
    pub child_message: AgentMessageSnapshot,
}

#[derive(Debug, Clone)]
pub struct EnsureCanonicalSubagentThreadInput {
    pub canonical_binding_key: String,
    pub capability: String,
    pub title: Option<String>,
    pub scope: AgentScope,
    pub bound_resource: Option<AgentResourceRef>,
    pub default_model_config_id: Option<i64>,
}

impl AgentApplication {
    /// Claims one queued Run and loads its owner/thread in the same façade.
    pub(crate) async fn claim_worker_run(
        &self,
        worker_id: &str,
    ) -> Result<Option<AgentWorkerRun>, AppError> {
        let row = agent::claim_runnable_run(self.connection(), worker_id)
            .await
            .map_err(agent_db_error_to_app)?;
        let Some(run_row) = row else {
            return Ok(None);
        };
        let thread_row = agent::thread_by_id(self.connection(), run_row.thread_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::InvalidState {
                current: format!("run {} 缺少 thread {}", run_row.id, run_row.thread_id),
                operation: "领取 Agent run".into(),
            })?;
        let lease_token = run_row
            .lease_token
            .clone()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("run {} 领取后缺少 lease token", run_row.id),
                operation: "领取 Agent run".into(),
            })?;
        Ok(Some(AgentWorkerRun {
            run: run_row.into(),
            thread: thread_snapshot(thread_row)?,
            lease_token,
        }))
    }

    /// Converts expired leases into the explicit recovery terminal state.
    pub(crate) async fn recover_expired_runs(
        &self,
        limit: u64,
        error_message: Option<&str>,
    ) -> Result<Vec<run::Model>, AppError> {
        let rows = agent::expired_running_runs(self.connection(), limit)
            .await
            .map_err(agent_db_error_to_app)?;
        let mut recovered = Vec::with_capacity(rows.len());
        for row in rows {
            match agent::recover_expired_run(self.connection(), row.id, error_message).await {
                Ok(recovered_row) => recovered.push(recovered_row),
                // Another worker may win the race between the read-only scan
                // and the fenced recovery.  Surface all other failures.
                Err(AgentDbError::Conflict(_)) => {}
                Err(error) => return Err(agent_db_error_to_app(error)),
            }
        }
        Ok(recovered)
    }

    pub(crate) async fn expire_pending_approvals(
        &self,
        limit: u64,
    ) -> Result<Vec<approval::Model>, AppError> {
        agent::expire_pending_approvals(self.connection(), limit)
            .await
            .map_err(agent_db_error_to_app)
    }

    async fn delegation_depth_for_run(
        &self,
        run_id: i64,
        parent_run_id: Option<i64>,
    ) -> Result<u8, AppError> {
        let mut depth = 0_u8;
        let mut ancestor = parent_run_id;
        let mut seen = HashSet::new();
        while let Some(parent_id) = ancestor {
            if !seen.insert(parent_id) {
                return Err(AppError::InvalidState {
                    current: format!("Agent run lineage cycle detected: {run_id}"),
                    operation: "读取 Agent run lineage".into(),
                });
            }
            depth = depth.saturating_add(1);
            ancestor = agent::run_by_id(self.connection(), parent_id)
                .await
                .map_err(agent_db_error_to_app)?
                .and_then(|row| row.parent_run_id);
        }
        Ok(depth)
    }

    /// Loads the complete durable context required for one model decision.
    pub(crate) async fn load_run_context(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<AgentRunContext, AppError> {
        let run_row = agent::run_by_id(self.connection(), claimed.run.id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent run 不存在: {}", claimed.run.id)))?;
        agent::validate_live_run_lease(
            &run_row,
            &claimed.lease_token,
            chrono::Utc::now().naive_utc(),
        )
        .map_err(|error| match error {
            AgentDbError::Conflict(message) => AppError::InvalidState {
                current: message,
                operation: "加载 Agent worker context".into(),
            },
            other => agent_db_error_to_app(other),
        })?;
        let thread_row = agent::thread_by_id(self.connection(), run_row.thread_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| {
                AppError::NotFound(format!("Agent thread 不存在: {}", run_row.thread_id))
            })?;
        let turn_row = turn::Entity::find_by_id(run_row.turn_id)
            .one(self.connection())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("Agent turn 不存在: {}", run_row.turn_id)))?;
        validate_model_override_json(turn_row.model_override_json.as_ref())?;
        let mut messages =
            agent::messages_by_thread(self.connection(), run_row.thread_id, SNAPSHOT_LIMIT)
                .await
                .map_err(agent_db_error_to_app)?;
        // The UI history limit must never drop a pending call from an active Run.
        messages.extend(
            agent::messages_by_run(self.connection(), run_row.id)
                .await
                .map_err(agent_db_error_to_app)?,
        );
        messages.sort_by_key(|message| message.id);
        messages.dedup_by_key(|message| message.id);
        let editor_source = messages
            .iter()
            .find(|message| message.id == turn_row.input_message_id)
            .and_then(|message| message.payload_json.as_ref())
            .and_then(|payload| payload.get("editor_source"))
            .cloned()
            .map(|value| from_json(value, "agent_message.editor_source"))
            .transpose()?;
        let delegations = agent::delegations_by_thread(self.connection(), run_row.thread_id)
            .await
            .map_err(agent_db_error_to_app)?;
        let mut scope: AgentScope =
            from_json(thread_row.scope_json.clone(), "agent_thread.scope_json")?;
        apply_subagent_tool_policy(&thread_row.profile_id, &mut scope)?;
        if scope.allowed_tools.is_empty()
            && a4n_api_contract::agent::capability_has_restricted_tool_catalog(
                &thread_row.profile_id,
            )
        {
            scope.allowed_tools =
                a4n_api_contract::agent::agent_tools_for_capability(&thread_row.profile_id)
                    .iter()
                    .map(|tool| (*tool).to_string())
                    .collect();
        }
        let executions = agent::executions_by_runs(self.connection(), vec![run_row.id])
            .await
            .map_err(agent_db_error_to_app)?;
        let runtime_snapshot_required = runtime_snapshot_required(&run_row.usage_json, run_row.id)?;
        let delegation_depth = self
            .delegation_depth_for_run(run_row.id, run_row.parent_run_id)
            .await?;
        Ok(AgentRunContext {
            owner_user_id: thread_row.owner_user_id,
            profile_id: thread_row.profile_id.clone(),
            default_model_config_id: thread_row.default_model_config_id,
            execution_mode: turn_row.execution_mode.clone(),
            goal: turn_row.goal.clone(),
            budget: run_row.budget_json.clone(),
            usage: run_row.usage_json.clone(),
            runtime_snapshot_required,
            delegation_depth,
            run: run_row.into(),
            thread: thread_snapshot(thread_row)?,
            turn: turn_row.into(),
            messages: messages.into_iter().map(Into::into).collect(),
            delegations: delegations.into_iter().map(Into::into).collect(),
            executions: executions.into_iter().map(Into::into).collect(),
            editor_source,
            scope,
        })
    }

    pub(crate) async fn update_context_summary(
        &self,
        claimed: &AgentWorkerRun,
        context_summary: Value,
    ) -> Result<(), AppError> {
        agent::update_context_summary_for_run_lease(
            self.connection(),
            claimed.run.id,
            &claimed.lease_token,
            context_summary,
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn read_memory_for_run(
        &self,
        context: &AgentRunContext,
        input: a4n_api_contract::agent::ReadAgentMemoryInput,
    ) -> Result<a4n_api_contract::agent::AgentMemorySearchResult, AppError> {
        let thread_row = &context.thread;
        let limit = bounded_limit(input.limit, 20, 100, "limit")? as usize;
        let query = input
            .query
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_lowercase();
        let mut memories =
            agent::list_memory(self.connection(), thread_row.owner_user_id, None, None)
                .await
                .map_err(agent_db_error_to_app)?;
        memories.retain(|row| {
            row.status == "active"
                && match row.scope_kind.as_str() {
                    "user" => row.scope_key == thread_row.owner_user_id.to_string(),
                    "project" => thread_row
                        .project_id
                        .is_some_and(|project_id| row.scope_key == project_id.to_string()),
                    "thread" => row.scope_key == thread_row.id.to_string(),
                    _ => false,
                }
        });
        memories.sort_by(|left, right| {
            (&left.scope_kind, &left.scope_key, &left.memory_key)
                .cmp(&(&right.scope_kind, &right.scope_key, &right.memory_key))
                .then_with(|| right.revision.cmp(&left.revision))
        });
        memories.dedup_by(|left, right| {
            left.scope_kind == right.scope_kind
                && left.scope_key == right.scope_key
                && left.memory_key == right.memory_key
        });
        memories.retain(|row| {
            row.memory_key.to_lowercase().contains(&query)
                || row.content.to_lowercase().contains(&query)
        });
        let total = memories.len();
        memories.truncate(limit);
        Ok(a4n_api_contract::agent::AgentMemorySearchResult {
            memories: memories
                .into_iter()
                .map(memory_view)
                .collect::<Result<_, _>>()?,
            total,
        })
    }

    pub(crate) async fn renew_worker_lease(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<(), AppError> {
        agent::renew_run_lease(self.connection(), claimed.run.id, &claimed.lease_token)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn read_execution_result_for_run(
        &self,
        context: &AgentRunContext,
        input: a4n_api_contract::agent::ReadAgentExecutionResultInput,
    ) -> Result<a4n_api_contract::agent::AgentExecutionResultPage, AppError> {
        let limit = bounded_limit(input.limit, 2_000, 2_000, "limit")? as usize;
        let execution_id = parse_id(&input.execution_id, "execution_id")?;
        let execution = execution::Entity::find_by_id(execution_id)
            .one(self.connection())
            .await?
            .filter(|execution| {
                matches!(
                    execution.execution_kind.as_str(),
                    "tool_call" | "delegation"
                )
            })
            .ok_or_else(|| AppError::Forbidden("不能读取此工具结果".into()))?;
        let source_run = agent::run_by_id(self.connection(), execution.run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .filter(|run| run.thread_id == context.thread.id)
            .ok_or_else(|| AppError::Forbidden("不能读取其他 Thread 的工具结果".into()))?;
        self.owned_thread(context.owner_user_id, source_run.thread_id)
            .await?;
        let value = execution
            .result_json
            .as_ref()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("工具结果尚不可用，状态为 {}", execution.status),
                operation: "读取 Agent 工具结果".into(),
            })?;
        let selected = value.pointer(&input.pointer).ok_or_else(|| {
            AppError::InvalidArgument(format!("结果中不存在 JSON pointer: {}", input.pointer))
        })?;
        let text = selected
            .as_str()
            .map(str::to_string)
            .unwrap_or(serde_json::to_string(selected)?);
        let total_chars = text.chars().count();
        let offset = input.offset as usize;
        if offset > total_chars {
            return Err(AppError::InvalidArgument("offset 超出结果长度".into()));
        }
        let page = text.chars().skip(offset).take(limit).collect::<String>();
        let end = offset + page.chars().count();
        Ok(a4n_api_contract::agent::AgentExecutionResultPage {
            execution_id: input.execution_id,
            pointer: input.pointer,
            text: page,
            offset: input.offset,
            total_chars,
            next_offset: (end < total_chars)
                .then(|| u32::try_from(end))
                .transpose()
                .map_err(|_| AppError::InvalidArgument("结果长度超出读取范围".into()))?,
            complete: end == total_chars,
        })
    }

    pub(crate) async fn result_refs_for_run(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<Value, AppError> {
        let executions = agent::executions_by_runs(self.connection(), vec![claimed.run.id])
            .await
            .map_err(agent_db_error_to_app)?;
        let mut refs = Vec::<AgentResourceRef>::new();
        for execution in executions
            .into_iter()
            .filter(|execution| execution.status == "succeeded")
        {
            if let Some(value) = execution.result_refs_json {
                let items: Vec<AgentResourceRef> =
                    from_json(value, "agent_execution.result_refs_json")?;
                for item in items {
                    if item.kind == "optimization_source" {
                        continue;
                    }
                    refs.retain(|previous| previous.kind != item.kind || previous.id != item.id);
                    refs.push(item);
                }
            }
        }
        serde_json::to_value(refs).map_err(AppError::from)
    }

    pub(crate) async fn run_budget_snapshot(
        &self,
        run_id: i64,
    ) -> Result<agent::AgentBudgetSnapshot, AppError> {
        agent::run_budget_snapshot(self.connection(), run_id)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn create_prepared_execution(
        &self,
        input: WorkerExecutionInput,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        match agent::create_execution_for_run_lease(
            self.connection(),
            AgentExecutionInput {
                run_id: input.run_id,
                execution_key: input.execution_key,
                execution_kind: input.execution_kind,
                status: "prepared".into(),
                tool_name: input.tool_name,
                effect: input.effect,
                replay_policy: input.replay_policy,
                arguments_json: input.arguments_json,
                resource_preconditions_json: input.resource_preconditions_json,
            },
            &input.lease_token,
        )
        .await
        {
            Ok(execution) => Ok(Some(execution.into())),
            Err(AgentDbError::BudgetExhausted(_)) => Ok(None),
            Err(error) => Err(agent_db_error_to_app(error)),
        }
    }

    /// Starts a child Run on an existing canonical sub-agent thread and binds
    /// lineage before the child becomes visible to the worker claim query.
    pub(crate) async fn create_child_run(
        &self,
        claimed: &AgentWorkerRun,
        input: WorkerDelegationInput,
    ) -> Result<WorkerDelegationResult, AppError> {
        let capability = required_capability(input.capability)?;
        let objective = required_text(input.objective, "objective")?;
        let delegation_key = required_text(input.delegation_key, "delegation_key")?;
        let mut child_scope = input.scope;
        validate_scope(&child_scope)?;
        let parent_scope: AgentScope =
            from_json(claimed.thread.scope_json.clone(), "agent_thread.scope_json")?;
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
        child_scope.resource_kind = child_scope
            .resource_kind
            .or(parent_scope.resource_kind.clone());
        child_scope.resource_id = child_scope.resource_id.or(parent_scope.resource_id.clone());
        child_scope.project_id = child_scope.project_id.or(parent_scope.project_id);
        child_scope.chapter_id = child_scope.chapter_id.or(parent_scope.chapter_id);
        if parent_scope
            .project_id
            .is_some_and(|project_id| child_scope.project_id != Some(project_id))
            || parent_scope
                .chapter_id
                .is_some_and(|chapter_id| child_scope.chapter_id != Some(chapter_id))
            || (parent_scope.resource_kind.is_some()
                && (child_scope.resource_kind != parent_scope.resource_kind
                    || child_scope.resource_id != parent_scope.resource_id))
        {
            return Err(AppError::Forbidden(
                "child Agent scope 不能超出 parent Agent 资源范围".into(),
            ));
        }
        apply_subagent_capability_policy(&capability, &mut child_scope)?;
        inherit_subagent_allowlists(&parent_scope, &mut child_scope)?;
        apply_subagent_tool_policy(&capability, &mut child_scope)?;
        let child_thread = agent::thread_by_id(self.connection(), input.child_thread_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "Agent child thread 不存在: {}",
                    input.child_thread_id
                ))
            })?;
        if child_thread.owner_user_id != claimed.thread.owner_user_id
            || child_thread.thread_kind != "subagent"
            || child_thread.profile_id != capability
        {
            return Err(AppError::InvalidArgument(
                "Agent child thread 与委派 capability/owner 不一致".into(),
            ));
        }
        let child_resource = match (
            child_thread.resource_type.clone(),
            child_thread.resource_id.clone(),
        ) {
            (Some(kind), Some(id)) => Some(AgentResourceRef {
                kind,
                id,
                label: child_thread.title.clone(),
                revision: None,
                hash: None,
            }),
            (None, None) => None,
            _ => {
                return Err(AppError::InvalidState {
                    current: format!("Agent child thread {} 的资源绑定不完整", child_thread.id),
                    operation: "创建 Agent child delegation".into(),
                });
            }
        };
        let (project_id, chapter_id) = self
            .verify_thread_scope(
                claimed.thread.owner_user_id,
                &child_scope,
                child_resource.as_ref(),
            )
            .await?;
        child_scope.project_id = child_scope.project_id.or(project_id);
        child_scope.chapter_id = child_scope.chapter_id.or(chapter_id);
        validate_subagent_resource(&capability, &child_scope, child_resource.as_ref())?;
        let parent_turn = turn::Entity::find_by_id(claimed.run.turn_id)
            .one(self.connection())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| {
                AppError::NotFound(format!("Agent parent turn 不存在: {}", claimed.run.turn_id))
            })?;
        validate_model_override_json(parent_turn.model_override_json.as_ref())?;
        let runtime_snapshot_required =
            runtime_snapshot_required(&claimed.run.usage_json, claimed.run.id)?;
        let scope_json = serde_json::to_value(&child_scope)?;
        let client_message_id = format!("delegation:{delegation_key}");
        let (delegation, child_turn, child_run, child_message) = agent::create_child_delegation(
            self.connection(),
            a4n_db::agent::AgentChildDelegationInput {
                parent_lease_token: Some(claimed.lease_token.clone()),
                delegation: AgentDelegationInput {
                    parent_thread_id: claimed.run.thread_id,
                    parent_run_id: claimed.run.id,
                    parent_execution_id: input.parent_execution_id,
                    child_thread_id: child_thread.id,
                    child_turn_id: None,
                    child_run_id: None,
                    delegation_key,
                    capability,
                    objective: objective.clone(),
                    success_criteria_json: serde_json::to_value(input.success_criteria)?,
                    scope_json,
                    budget_json: input.budget_json.clone(),
                    status: "pending".into(),
                },
                child_turn: AgentTurnInput {
                    thread_id: child_thread.id,
                    client_message_id: client_message_id.clone(),
                    message: AgentMessageInput {
                        thread_id: child_thread.id,
                        turn_id: None,
                        run_id: None,
                        role: "user".into(),
                        sender_kind: "agent".into(),
                        sender_thread_id: Some(claimed.run.thread_id),
                        client_message_id: Some(client_message_id),
                        content: objective.clone(),
                        status: "complete".into(),
                        payload_json: Some(json!({
                            "parent_run_id": claimed.run.id.to_string(),
                            "delegation_key": format!("{}:{}", claimed.run.id, child_thread.id),
                        })),
                    },
                    execution_mode: parent_turn.execution_mode.clone(),
                    model_override_json: parent_turn.model_override_json.clone(),
                    budget_json: input.budget_json,
                    runtime_snapshot_required,
                    trigger: "delegation".into(),
                    goal: Some(objective),
                },
            },
        )
        .await
        .map_err(agent_db_error_to_app)?;
        Ok(WorkerDelegationResult {
            delegation: delegation.into(),
            child_turn: child_turn.into(),
            child_run: child_run.into(),
            child_message: child_message.into(),
        })
    }

    pub(crate) async fn complete_child_run(
        &self,
        claimed: &AgentWorkerRun,
        content: impl Into<String>,
        result_summary: impl Into<String>,
        result_refs_json: Option<Value>,
        observed_input_message_id: i64,
    ) -> Result<bool, AppError> {
        if claimed.run.parent_run_id.is_none() {
            return Err(AppError::InvalidArgument(
                "只有 child Agent run 可以回传 delegation result".into(),
            ));
        }
        agent::complete_child_delegation(
            self.connection(),
            a4n_db::agent::AgentChildCompletionInput {
                child_run_id: claimed.run.id,
                child_lease_token: claimed.lease_token.clone(),
                final_message: AgentMessageInput {
                    thread_id: claimed.run.thread_id,
                    turn_id: Some(claimed.run.turn_id),
                    run_id: Some(claimed.run.id),
                    role: "assistant".into(),
                    sender_kind: "agent".into(),
                    sender_thread_id: None,
                    client_message_id: Some(format!("agent-final:{}", claimed.run.id)),
                    content: content.into(),
                    status: "complete".into(),
                    payload_json: None,
                },
                result_refs_json,
                result_summary: result_summary.into(),
                observed_input_message_id,
            },
        )
        .await
        .map(|result| result.is_some())
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn update_worker_execution(
        &self,
        input: WorkerExecutionUpdate,
    ) -> Result<AgentExecutionSnapshot, AppError> {
        agent::update_execution_for_run_lease(
            self.connection(),
            input.run_id,
            &input.lease_token,
            input.execution_id,
            &input.status,
            input.result_json,
            input.result_summary,
            input.result_refs_json,
            input.error_message,
        )
        .await
        .map(Into::into)
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn update_run_usage(
        &self,
        input: WorkerUsage,
    ) -> Result<run::Model, AppError> {
        agent::update_run_usage(
            self.connection(),
            input.run_id,
            &input.lease_token,
            input.usage_json,
        )
        .await
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn mark_turn_active(&self, claimed: &AgentWorkerRun) -> Result<(), AppError> {
        agent::mark_turn_active_for_run_lease(
            self.connection(),
            claimed.run.id,
            &claimed.lease_token,
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn wait_run(
        &self,
        claimed: &AgentWorkerRun,
        reason: &str,
    ) -> Result<(), AppError> {
        validate_waiting_reason(reason)?;
        agent::wait_run_for_lease(
            self.connection(),
            claimed.run.id,
            &claimed.lease_token,
            reason,
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn wait_for_editor(
        &self,
        claimed: &AgentWorkerRun,
        request: &a4n_api_contract::agent::AgentEditorRequest,
    ) -> Result<(), AppError> {
        agent::wait_run_for_lease_with_request(
            self.connection(),
            claimed.run.id,
            &claimed.lease_token,
            "external",
            Some(serde_json::to_value(request)?),
        )
        .await
        .map_err(agent_db_error_to_app)?;
        Ok(())
    }

    pub(crate) async fn claim_editor_request(
        &self,
        user_id: i64,
        input: &a4n_api_contract::agent::RespondAgentEditorInput,
    ) -> Result<(AgentWorkerRun, a4n_api_contract::agent::AgentEditorRequest), AppError> {
        let run_id = parse_id(&input.run_id, "run_id")?;
        let execution_id = parse_id(&input.execution_id, "execution_id")?;
        let run = agent::run_by_id(self.connection(), run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound("Agent Run 不存在".into()))?;
        let thread = self.owned_thread(user_id, run.thread_id).await?;
        let payload = event::Entity::find()
            .filter(event::Column::RunId.eq(run_id))
            .filter(event::Column::Kind.eq("run_waiting"))
            .order_by_desc(event::Column::Sequence)
            .one(self.connection())
            .await?
            .and_then(|event| event.payload_json)
            .and_then(|payload| payload.get("editor_request").cloned())
            .ok_or_else(|| AppError::InvalidArgument("Run 没有待响应的编辑器请求".into()))?;
        let request: a4n_api_contract::agent::AgentEditorRequest =
            from_json(payload, "editor_request")?;
        if request.execution_id != input.execution_id
            || request.source.view_instance_id != input.view_instance_id
        {
            return Err(AppError::Forbidden("响应与原始视图或执行不匹配".into()));
        }
        validate_editor_source(&request.source, &thread)?;
        let run = agent::claim_editor_execution(self.connection(), user_id, run_id, execution_id)
            .await
            .map_err(agent_db_error_to_app)?;
        let lease_token = run
            .lease_token
            .clone()
            .ok_or_else(|| AppError::InvalidState {
                current: "编辑器响应缺少执行租约".into(),
                operation: "领取编辑器请求".into(),
            })?;
        Ok((
            AgentWorkerRun {
                run: run.into(),
                thread: thread_snapshot(thread)?,
                lease_token,
            },
            request,
        ))
    }

    pub(crate) async fn requeue_after_external_response(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<(), AppError> {
        agent::update_run_state(
            self.connection(),
            claimed.run.id,
            Some(&claimed.lease_token),
            "queued",
            None,
            None,
        )
        .await
        .map_err(agent_db_error_to_app)?;
        Ok(())
    }

    pub(crate) async fn append_worker_update(
        &self,
        claimed: &AgentWorkerRun,
        kind: &str,
        status: &str,
        public_text: impl Into<String>,
        payload_json: Option<Value>,
    ) -> Result<event::Model, AppError> {
        agent::append_event_for_run_lease(
            self.connection(),
            claimed.run.id,
            &claimed.lease_token,
            AgentEventInput {
                thread_id: claimed.run.thread_id,
                turn_id: Some(claimed.run.turn_id),
                run_id: Some(claimed.run.id),
                execution_id: None,
                delegation_id: None,
                message_id: None,
                actor_kind: "agent".into(),
                actor_id: Some(claimed.run.id.to_string()),
                kind: kind.to_string(),
                status: status.to_string(),
                public_text: public_text.into(),
                payload_json,
            },
        )
        .await
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn append_tool_message(
        &self,
        claimed: &AgentWorkerRun,
        execution_id: i64,
        tool_name: &str,
        tool_call_id: Option<&str>,
        result: Value,
    ) -> Result<AgentMessageSnapshot, AppError> {
        let content = serde_json::to_string(&result)?;
        agent::append_message_for_run_lease(
            self.connection(),
            claimed.run.id,
            &claimed.lease_token,
            AgentMessageInput {
                thread_id: claimed.run.thread_id,
                turn_id: Some(claimed.run.turn_id),
                run_id: Some(claimed.run.id),
                role: "tool".into(),
                sender_kind: "tool".into(),
                sender_thread_id: None,
                client_message_id: Some(format!("agent-tool:{execution_id}")),
                content,
                status: "complete".into(),
                payload_json: Some(json!({
                    "execution_id": execution_id.to_string(),
                    "tool": tool_name,
                    "tool_call_id": tool_call_id,
                    "result": result,
                })),
            },
        )
        .await
        .map(Into::into)
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn append_assistant_tool_message(
        &self,
        claimed: &AgentWorkerRun,
        content: impl Into<String>,
        tool_calls: Value,
    ) -> Result<AgentMessageSnapshot, AppError> {
        agent::append_message_for_run_lease(
            self.connection(),
            claimed.run.id,
            &claimed.lease_token,
            AgentMessageInput {
                thread_id: claimed.run.thread_id,
                turn_id: Some(claimed.run.turn_id),
                run_id: Some(claimed.run.id),
                role: "assistant".into(),
                sender_kind: "agent".into(),
                sender_thread_id: None,
                client_message_id: Some(format!(
                    "agent-assistant:{}:{}",
                    claimed.run.id, tool_calls
                )),
                content: content.into(),
                status: "complete".into(),
                payload_json: Some(json!({ "tool_calls": tool_calls })),
            },
        )
        .await
        .map(Into::into)
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn upsert_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        content: String,
        reasoning_content: Option<String>,
    ) -> Result<AgentMessageSnapshot, AppError> {
        agent::upsert_assistant_streaming_message_for_run_lease(
            self.connection(),
            claimed.run.id,
            &claimed.lease_token,
            AgentMessageInput {
                thread_id: claimed.run.thread_id,
                turn_id: Some(claimed.run.turn_id),
                run_id: Some(claimed.run.id),
                role: "assistant".into(),
                sender_kind: "agent".into(),
                sender_thread_id: None,
                client_message_id: Some(client_message_id),
                content,
                status: "streaming".into(),
                payload_json: Some(json!({
                    "streaming": true,
                    "reasoning_content": reasoning_content,
                })),
            },
        )
        .await
        .map(Into::into)
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn interrupt_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        reason: Option<String>,
    ) -> Result<(), AppError> {
        agent::interrupt_assistant_streaming_message_for_run_lease(
            self.connection(),
            claimed.run.id,
            &claimed.lease_token,
            &client_message_id,
            reason.as_deref(),
        )
        .await
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn complete_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        content: String,
        tool_calls: Value,
    ) -> Result<Option<AgentMessageSnapshot>, AppError> {
        agent::complete_assistant_streaming_message_for_run_lease(
            self.connection(),
            claimed.run.id,
            &claimed.lease_token,
            &client_message_id,
            content,
            json!({ "tool_calls": tool_calls }),
        )
        .await
        .map(|message| message.map(Into::into))
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn complete_claimed_run(
        &self,
        claimed: &AgentWorkerRun,
        content: impl Into<String>,
        result_refs_json: Option<Value>,
        observed_input_message_id: i64,
    ) -> Result<bool, AppError> {
        let completed = agent::finish_run_with_message(
            self.connection(),
            claimed.run.id,
            &claimed.lease_token,
            AgentMessageInput {
                thread_id: claimed.run.thread_id,
                turn_id: Some(claimed.run.turn_id),
                run_id: Some(claimed.run.id),
                role: "assistant".into(),
                sender_kind: "agent".into(),
                sender_thread_id: None,
                client_message_id: Some(format!("agent-final:{}", claimed.run.id)),
                content: content.into(),
                status: "complete".into(),
                payload_json: None,
            },
            result_refs_json,
            observed_input_message_id,
        )
        .await
        .map_err(agent_db_error_to_app)?;
        Ok(completed.is_some())
    }

    async fn fail_claimed_run(
        &self,
        claimed: &AgentWorkerRun,
        error_message: impl Into<String>,
    ) -> Result<run::Model, AppError> {
        let error_message = error_message.into();
        agent::fail_run_for_lease(
            self.connection(),
            claimed.run.id,
            &claimed.lease_token,
            Some(error_message),
        )
        .await
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn fail_child_run(
        &self,
        claimed: &AgentWorkerRun,
        error_message: impl Into<String>,
    ) -> Result<(), AppError> {
        if claimed.run.parent_run_id.is_none() {
            self.fail_claimed_run(claimed, error_message).await?;
            return Ok(());
        }
        agent::fail_child_delegation(
            self.connection(),
            a4n_db::agent::AgentChildFailureInput {
                child_run_id: claimed.run.id,
                child_lease_token: claimed.lease_token.clone(),
                error_message: error_message.into(),
            },
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn claim_inbox_for_thread(
        &self,
        thread_id: i64,
    ) -> Result<Option<AgentInboxSnapshot>, AppError> {
        agent::claim_inbox(self.connection(), Some(thread_id))
            .await
            .map(|row| row.map(Into::into))
            .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn consume_claimed_inbox(
        &self,
        row: &AgentInboxSnapshot,
    ) -> Result<AgentInboxSnapshot, AppError> {
        let token = row
            .claim_token
            .as_deref()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("inbox {} 缺少 claim token", row.id),
                operation: "消费 Agent inbox".into(),
            })?;
        agent::consume_inbox(self.connection(), row.id, token)
            .await
            .map(Into::into)
            .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn persist_claimed_inbox(
        &self,
        claimed: &AgentWorkerRun,
        row: &AgentInboxSnapshot,
    ) -> Result<(AgentInboxSnapshot, AgentMessageSnapshot), AppError> {
        if row.thread_id != claimed.run.thread_id
            || row.turn_id != Some(claimed.run.turn_id)
            || row.run_id != Some(claimed.run.id)
        {
            return Err(AppError::InvalidArgument(
                "Agent inbox 不属于当前 worker run".into(),
            ));
        }
        let token = row
            .claim_token
            .as_deref()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("inbox {} 缺少 claim token", row.id),
                operation: "持久化 Agent inbox".into(),
            })?;
        agent::consume_inbox_with_message(
            self.connection(),
            row.id,
            token,
            &claimed.lease_token,
            AgentMessageInput {
                thread_id: row.thread_id,
                turn_id: row.turn_id,
                run_id: row.run_id,
                role: "user".into(),
                sender_kind: row.source_kind.clone(),
                sender_thread_id: row.source_thread_id,
                client_message_id: Some(format!("agent-inbox:{}", row.id)),
                content: row.content.clone(),
                status: "complete".into(),
                payload_json: Some(json!({
                    "inbox_id": row.id.to_string(),
                    "inbox_kind": row.inbox_kind.clone(),
                    "delivery_mode": row.delivery_mode.clone(),
                    "payload": row.payload_json.clone(),
                })),
            },
        )
        .await
        .map(|(inbox, message)| (inbox.into(), message.into()))
        .map_err(agent_db_error_to_app)
    }

    /// Returns an approved operation for the given execution.  Approval
    /// consumption is intentionally left to the side-effect owner; an
    /// approved row is evidence, not permission to mutate unrelated data.
    pub(crate) async fn approved_execution(
        &self,
        thread_id: i64,
        execution_id: i64,
    ) -> Result<Option<approval::Model>, AppError> {
        Ok(agent::approvals_by_thread(self.connection(), thread_id)
            .await
            .map_err(agent_db_error_to_app)?
            .into_iter()
            .find(|row| row.execution_id == execution_id && row.status == "approved"))
    }

    /// Atomically records a write approval request and releases the claimed
    /// Run into `waiting(approval)`. Runtime workers must use this façade
    /// instead of composing the two database operations themselves.
    pub(crate) async fn request_worker_approval(
        &self,
        claimed: &AgentWorkerRun,
        execution_id: i64,
        operation_json: Value,
        resource_preconditions_json: Option<Value>,
        expires_at: Option<chrono::NaiveDateTime>,
    ) -> Result<approval::Model, AppError> {
        let operation_digest = a4n_db::agent::sha256_hex(
            &serde_json::to_string(&operation_json).map_err(AppError::Serialization)?,
        );
        let (approval, _, _) = agent::request_approval_for_run_lease(
            self.connection(),
            &claimed.lease_token,
            a4n_domain::agent::AgentApprovalInput {
                thread_id: claimed.run.thread_id,
                run_id: claimed.run.id,
                execution_id,
                operation_digest,
                operation_json,
                resource_preconditions_json,
                expires_at,
            },
        )
        .await
        .map_err(agent_db_error_to_app)?;
        Ok(approval)
    }

    pub(crate) async fn mark_recovery_required(
        &self,
        claimed: &AgentWorkerRun,
        reason: impl Into<String>,
    ) -> Result<run::Model, AppError> {
        agent::recovery_required_for_lease(
            self.connection(),
            claimed.run.id,
            &claimed.lease_token,
            Some(reason.into()),
        )
        .await
        .map_err(agent_db_error_to_app)
    }
}

fn validate_user_id(user_id: i64) -> Result<(), AppError> {
    if user_id > 0 {
        Ok(())
    } else {
        Err(AppError::InvalidArgument("user_id 无效".into()))
    }
}

fn validate_waiting_reason(reason: &str) -> Result<(), AppError> {
    match reason {
        "user" | "approval" | "child" | "external" | "backoff" | "budget" | "reconcile" => Ok(()),
        _ => Err(AppError::InvalidArgument(format!(
            "未知 Agent waiting reason: {reason}"
        ))),
    }
}

fn validate_optional_positive_id(value: Option<i64>, field: &str) -> Result<(), AppError> {
    if value.is_some_and(|id| id <= 0) {
        Err(AppError::InvalidArgument(format!("{field} 无效")))
    } else {
        Ok(())
    }
}

fn admit_new_run_runtime_snapshot(
    runtime_snapshot_required: bool,
    require_runtime_snapshot_for_new_run: bool,
) -> Result<(), AppError> {
    if require_runtime_snapshot_for_new_run && !runtime_snapshot_required {
        return Err(AppError::InvalidArgument(
            "创建 Agent run 必须提供 runtime_config_snapshot".into(),
        ));
    }
    Ok(())
}

fn validate_model_override_json(value: Option<&Value>) -> Result<(), AppError> {
    let Some(value) = value else {
        return Ok(());
    };
    let object = value.as_object().ok_or_else(|| AppError::InvalidState {
        current: "Agent turn model_override_json 必须是 object".into(),
        operation: "加载 Agent 模型配置".into(),
    })?;
    if object.len() != 1 {
        return Err(AppError::InvalidState {
            current: "Agent turn model_override_json 只能包含 model_config_id".into(),
            operation: "加载 Agent 模型配置".into(),
        });
    }
    object
        .get("model_config_id")
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
        .ok_or_else(|| AppError::InvalidState {
            current: "Agent turn model_override_json.model_config_id 无效".into(),
            operation: "加载 Agent 模型配置".into(),
        })?;
    Ok(())
}

fn validate_editor_source(
    source: &a4n_api_contract::agent::AgentEditorSource,
    thread: &thread::Model,
) -> Result<(), AppError> {
    let scope: AgentScope = from_json(thread.scope_json.clone(), "agent_thread.scope_json")?;
    if thread.profile_id != a4n_api_contract::agent::DIRECTOR_AGENT_CAPABILITY
        || scope.project_id != Some(source.project_id)
        || scope.chapter_id != Some(source.chapter_id)
    {
        return Err(AppError::Forbidden(
            "编辑器来源必须属于当前导演 Agent 绑定的项目和章节".into(),
        ));
    }
    for value in [
        Some(&source.view_instance_id),
        Some(&source.editor_revision),
        source.scene_id.as_ref(),
        source.shot_id.as_ref(),
        source.keyframe_id.as_ref(),
        source.entity_id.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if value.trim().is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
            return Err(AppError::InvalidArgument("编辑器来源标识无效".into()));
        }
    }
    Ok(())
}

fn parse_id(value: &str, field: &str) -> Result<i64, AppError> {
    value
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| AppError::InvalidArgument(format!("{field} 无效")))
}

fn required_text(value: String, field: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(format!("{field} 不能为空")))
    } else {
        Ok(value.to_string())
    }
}

fn optional_text(value: Option<String>, field: &str) -> Result<Option<String>, AppError> {
    value.map(|value| required_text(value, field)).transpose()
}

fn validate_scope(scope: &AgentScope) -> Result<(), AppError> {
    if scope.project_id.is_some_and(|id| id <= 0) || scope.chapter_id.is_some_and(|id| id <= 0) {
        return Err(AppError::InvalidArgument("Agent scope 中的 id 无效".into()));
    }
    if scope.resource_kind.is_some() != scope.resource_id.is_some() {
        return Err(AppError::InvalidArgument(
            "Agent scope 的 resource_kind/resource_id 必须同时提供".into(),
        ));
    }
    Ok(())
}

fn validate_resource_ref(resource: &AgentResourceRef) -> Result<(), AppError> {
    if resource.kind.trim().is_empty() || resource.id.trim().is_empty() {
        Err(AppError::InvalidArgument(
            "bound_resource.kind/id 不能为空".into(),
        ))
    } else {
        Ok(())
    }
}

fn required_capability(value: String) -> Result<String, AppError> {
    let value = required_text(value, "capability")?;
    if value.len() > 128 || value.chars().any(|ch| ch.is_control()) {
        return Err(AppError::InvalidArgument(
            "capability 长度或字符无效".into(),
        ));
    }
    Ok(value)
}

fn inferred_subagent_resource(
    capability: &str,
    scope: &AgentScope,
) -> Result<Option<AgentResourceRef>, AppError> {
    let resource = match capability {
        a4n_api_contract::agent::CONTENT_OPTIMIZATION_AGENT_CAPABILITY => {
            let chapter_id = scope.chapter_id.ok_or_else(|| {
                AppError::InvalidArgument("content_optimization child 必须绑定 chapterId".into())
            })?;
            Some(AgentResourceRef {
                kind: "chapter".into(),
                id: chapter_id.to_string(),
                label: None,
                revision: None,
                hash: None,
            })
        }
        a4n_api_contract::agent::CANVAS_AGENT_CAPABILITY => {
            let project_id = scope.project_id.ok_or_else(|| {
                AppError::InvalidArgument("canvas child 必须绑定 projectId".into())
            })?;
            Some(AgentResourceRef {
                kind: "project".into(),
                id: project_id.to_string(),
                label: None,
                revision: None,
                hash: None,
            })
        }
        a4n_api_contract::agent::DIRECTOR_AGENT_CAPABILITY => {
            let chapter_id = scope.chapter_id.ok_or_else(|| {
                AppError::InvalidArgument("director child 必须绑定 chapterId".into())
            })?;
            Some(AgentResourceRef {
                kind: "chapter".into(),
                id: chapter_id.to_string(),
                label: None,
                revision: None,
                hash: None,
            })
        }
        _ => match (scope.resource_kind.as_deref(), scope.resource_id.as_deref()) {
            (Some(kind), Some(id)) => Some(AgentResourceRef {
                kind: kind.to_string(),
                id: id.to_string(),
                label: None,
                revision: None,
                hash: None,
            }),
            (None, None) if scope.chapter_id.is_some() => Some(AgentResourceRef {
                kind: "chapter".into(),
                id: scope.chapter_id.expect("checked Some").to_string(),
                label: None,
                revision: None,
                hash: None,
            }),
            (None, None) if scope.project_id.is_some() => Some(AgentResourceRef {
                kind: "project".into(),
                id: scope.project_id.expect("checked Some").to_string(),
                label: None,
                revision: None,
                hash: None,
            }),
            (None, None) => None,
            _ => {
                return Err(AppError::InvalidArgument(
                    "Agent scope 的 resource_kind/resource_id 必须同时提供".into(),
                ));
            }
        },
    };
    Ok(resource)
}

fn validate_subagent_resource(
    capability: &str,
    scope: &AgentScope,
    resource: Option<&AgentResourceRef>,
) -> Result<(), AppError> {
    match capability {
        a4n_api_contract::agent::CONTENT_OPTIMIZATION_AGENT_CAPABILITY
            if resource.is_none_or(|resource| resource.kind != "chapter")
                || scope.project_id.is_none()
                || scope.chapter_id.is_none() =>
        {
            return Err(AppError::InvalidArgument(
                "content_optimization child 必须绑定 project/chapter".into(),
            ));
        }
        a4n_api_contract::agent::CANVAS_AGENT_CAPABILITY
            if resource.is_none_or(|resource| resource.kind != "project")
                || scope.project_id.is_none()
                || scope.chapter_id.is_some() =>
        {
            return Err(AppError::InvalidArgument(
                "canvas child 只能绑定 project，不能绑定 chapter".into(),
            ));
        }
        a4n_api_contract::agent::DIRECTOR_AGENT_CAPABILITY
            if resource.is_none_or(|resource| resource.kind != "chapter")
                || scope.project_id.is_none()
                || scope.chapter_id.is_none() =>
        {
            return Err(AppError::InvalidArgument(
                "director child 必须绑定 project/chapter".into(),
            ));
        }
        _ => {}
    }
    Ok(())
}

/// Apply the one tool-scope rule shared by direct thread creation and
/// parent-driven delegation. Empty means the capability's current catalog;
/// only the worker materializes that catalog, so stored defaults do not go
/// stale. Existing nonempty lists remain explicit restrictions.
fn apply_subagent_tool_policy(capability: &str, scope: &mut AgentScope) -> Result<(), AppError> {
    if a4n_api_contract::agent::capability_has_restricted_tool_catalog(capability) {
        let allowed_catalog = a4n_api_contract::agent::agent_tools_for_capability(capability);
        for tool in &scope.allowed_tools {
            if !allowed_catalog.contains(&tool.as_str()) {
                return Err(AppError::Forbidden(format!(
                    "capability {capability} 不允许使用工具: {tool}"
                )));
            }
        }
        return Ok(());
    }

    for tool in &scope.allowed_tools {
        let descriptor = a4n_api_contract::agent::synthetic_agent_tool_descriptor(tool)
            .or_else(|| a4n_api_contract::command_manifest::command_descriptor(tool))
            .ok_or_else(|| AppError::Forbidden(format!("Agent 工具不存在或不可发现: {tool}")))?;
        if descriptor
            .agent_policy
            .as_ref()
            .is_none_or(|policy| policy.exposure == AgentToolExposure::Hidden)
        {
            return Err(AppError::Forbidden(format!(
                "Agent 工具不允许暴露给通用子 Agent: {tool}"
            )));
        }
    }
    Ok(())
}

/// Capability allowlists describe which profiles a run may delegate to. An
/// empty list is intentionally open; registered domain children are the only
/// profiles forced into a single-capability boundary.
fn apply_subagent_capability_policy(
    capability: &str,
    scope: &mut AgentScope,
) -> Result<(), AppError> {
    if a4n_api_contract::agent::capability_has_restricted_tool_catalog(capability) {
        if scope.allowed_capabilities.is_empty() {
            scope.allowed_capabilities = vec![capability.to_string()];
        } else if scope
            .allowed_capabilities
            .iter()
            .any(|allowed| allowed != capability)
        {
            return Err(AppError::Forbidden(format!(
                "capability {capability} 只能委派同一 capability"
            )));
        } else {
            scope.allowed_capabilities = vec![capability.to_string()];
        }
    }
    Ok(())
}

/// Child runs must not widen a parent's explicit capability/tool allowlists.
/// Empty means unrestricted and therefore inherits no additional fence.
pub fn inherit_subagent_allowlists(
    parent: &AgentScope,
    child: &mut AgentScope,
) -> Result<(), AppError> {
    if !parent.allowed_capabilities.is_empty() {
        if child.allowed_capabilities.is_empty() {
            child.allowed_capabilities = parent.allowed_capabilities.clone();
        } else if child.allowed_capabilities.iter().any(|allowed| {
            !parent
                .allowed_capabilities
                .iter()
                .any(|item| item == allowed)
        }) {
            return Err(AppError::Forbidden(
                "child Agent capability allowlist 不能超出 parent Agent".into(),
            ));
        }
    }
    if !parent.allowed_tools.is_empty() {
        if child.allowed_tools.is_empty() {
            child.allowed_tools = parent.allowed_tools.clone();
        } else if child
            .allowed_tools
            .iter()
            .any(|allowed| !parent.allowed_tools.iter().any(|item| item == allowed))
        {
            return Err(AppError::Forbidden(
                "child Agent tool allowlist 不能超出 parent Agent".into(),
            ));
        }
    }
    Ok(())
}

fn canonical_subagent_binding_key(
    capability: &str,
    resource: &AgentResourceRef,
) -> Result<String, AppError> {
    let capability = required_text(capability.to_string(), "capability")?;
    let kind = required_text(resource.kind.clone(), "bound_resource.kind")?;
    let id = required_text(resource.id.clone(), "bound_resource.id")?;
    Ok(a4n_api_contract::agent::canonical_subagent_binding_key(
        &capability,
        &kind,
        &id,
    ))
}

fn collect_scoped_resource(
    kind: &str,
    id: &str,
    project_id: &mut Option<i64>,
    chapter_id: &mut Option<i64>,
) -> Result<(), AppError> {
    match kind.trim() {
        "project" => merge_scope_id(
            project_id,
            parse_id(id, "project resource id")?,
            "project_id",
        ),
        "chapter" => merge_scope_id(
            chapter_id,
            parse_id(id, "chapter resource id")?,
            "chapter_id",
        ),
        _ => Ok(()),
    }
}

fn merge_scope_id(target: &mut Option<i64>, value: i64, field: &str) -> Result<(), AppError> {
    match *target {
        Some(existing) if existing != value => Err(AppError::InvalidArgument(format!(
            "Agent scope 中的 {field} 相互冲突: {existing} != {value}"
        ))),
        Some(_) => Ok(()),
        None => {
            *target = Some(value);
            Ok(())
        }
    }
}

fn bounded_limit(value: Option<u32>, default: u32, max: u32, field: &str) -> Result<u32, AppError> {
    let value = value.unwrap_or(default);
    if value == 0 {
        return Err(AppError::InvalidArgument(format!("{field} 必须大于 0")));
    }
    Ok(value.min(max))
}

fn normalized_capability(
    kind: AgentThreadKind,
    capability: Option<String>,
) -> Result<String, AppError> {
    match kind {
        AgentThreadKind::Main => match capability {
            Some(value) if value.trim() != MAIN_AGENT_CAPABILITY => Err(AppError::InvalidArgument(
                "主 Agent capability 必须为 main".into(),
            )),
            _ => Ok(MAIN_AGENT_CAPABILITY.into()),
        },
        AgentThreadKind::Subagent => required_capability(
            capability
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(GENERAL_AGENT_CAPABILITY)
                .to_string(),
        ),
    }
}

fn enum_value<T: serde::Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .expect("agent enums serialize")
        .as_str()
        .expect("agent enums serialize to strings")
        .to_string()
}

fn delivery_inbox_kind(delivery: AgentDeliveryMode) -> AgentInboxKind {
    match delivery {
        AgentDeliveryMode::Followup => AgentInboxKind::Followup,
        AgentDeliveryMode::Steer => AgentInboxKind::Steer,
        AgentDeliveryMode::Inject => AgentInboxKind::Inject,
    }
}

fn delivery_priority(delivery: AgentDeliveryMode) -> i32 {
    match delivery {
        AgentDeliveryMode::Followup => 0,
        AgentDeliveryMode::Steer => 10,
        AgentDeliveryMode::Inject => 20,
    }
}

fn validate_control_transition(status: &str, action: AgentControlAction) -> Result<(), AppError> {
    let allowed = match action {
        AgentControlAction::Pause => matches!(status, "queued" | "running" | "waiting"),
        AgentControlAction::Resume => matches!(status, "paused" | "pausing"),
        AgentControlAction::Cancel => !matches!(
            status,
            "completed" | "failed" | "cancelled" | "interrupted" | "recovery_required"
        ),
    };
    if allowed {
        Ok(())
    } else {
        Err(AppError::InvalidState {
            current: status.to_string(),
            operation: format!("{:?} Agent run", action).to_lowercase(),
        })
    }
}

fn accepted(
    thread_id: i64,
    message_id: i64,
    turn_id: i64,
    run_id: i64,
    status: &str,
    created_run: bool,
) -> Result<AgentMessageAccepted, AppError> {
    Ok(AgentMessageAccepted {
        thread_id: thread_id.to_string(),
        message_id: message_id.to_string(),
        turn_id: turn_id.to_string(),
        run_id: run_id.to_string(),
        status: parse_run_status(status)?,
        created_run,
    })
}

fn thread_view(row: thread::Model) -> Result<AgentThreadView, AppError> {
    let scope = from_json(row.scope_json, "agent_thread.scope_json")?;
    let bound_resource = match (row.resource_type, row.resource_id) {
        (Some(kind), Some(id)) => Some(AgentResourceRef {
            kind,
            id,
            label: None,
            revision: None,
            hash: None,
        }),
        (None, None) => None,
        _ => {
            return Err(AppError::InvalidState {
                current: format!("Agent thread {} 的资源绑定不完整", row.id),
                operation: "读取 Agent thread".into(),
            });
        }
    };
    let context_summary = parse_agent_context_summary(row.context_summary.as_deref())?;
    Ok(AgentThreadView {
        protocol_version: AGENT_API_PROTOCOL_VERSION,
        id: row.id.to_string(),
        owner_user_id: row.owner_user_id,
        kind: parse_thread_kind(&row.thread_kind)?,
        capability: Some(row.profile_id),
        title: row.title,
        bound_resource,
        scope,
        default_model_config_id: row.default_model_config_id,
        status: if row.archived_at.is_some() || row.status == "archived" {
            AgentThreadStatus::Archived
        } else {
            AgentThreadStatus::Active
        },
        revision: non_negative_u64(row.revision, "agent_thread.revision")?,
        next_turn_sequence: non_negative_u64(row.next_turn_sequence, "next_turn_sequence")?,
        next_event_sequence: non_negative_u64(row.next_event_sequence, "next_event_sequence")?,
        context_summary,
        created_at: row.created_at.to_string(),
        updated_at: row.updated_at.to_string(),
    })
}

fn message_view(row: message::Model) -> Result<AgentMessageView, AppError> {
    Ok(AgentMessageView {
        id: row.id.to_string(),
        thread_id: row.thread_id.to_string(),
        turn_id: row.turn_id.map(|id| id.to_string()),
        run_id: row.run_id.map(|id| id.to_string()),
        role: parse_message_role(&row.role)?,
        status: parse_message_status(&row.status)?,
        content: row.content,
        source_kind: Some(row.sender_kind),
        source_id: row.sender_thread_id.map(|id| id.to_string()),
        metadata: row.payload_json,
        created_at: row.created_at.to_string(),
        updated_at: Some(row.updated_at.to_string()),
    })
}

fn turn_view(row: turn::Model) -> Result<AgentTurnView, AppError> {
    Ok(AgentTurnView {
        id: row.id.to_string(),
        thread_id: row.thread_id.to_string(),
        sequence: non_negative_u64(row.sequence, "agent_turn.sequence")?,
        trigger: parse_turn_trigger(&row.trigger)?,
        goal: row.goal,
        status: parse_turn_status(&row.status)?,
        final_message_id: row.final_message_id.map(|id| id.to_string()),
        result_refs: optional_json_vec(row.result_refs_json, "agent_turn.result_refs_json")?,
        created_at: row.created_at.to_string(),
        updated_at: row.updated_at.to_string(),
    })
}

fn run_view(row: run::Model) -> Result<AgentRunView, AppError> {
    runtime_snapshot_required(&row.usage_json, row.id)?;
    let usage = row
        .usage_json
        .as_object()
        .ok_or_else(|| AppError::InvalidState {
            current: format!("Agent run {} usage_json 不是对象", row.id),
            operation: "读取 Agent run".into(),
        })?;
    Ok(AgentRunView {
        id: row.id.to_string(),
        thread_id: row.thread_id.to_string(),
        turn_id: row.turn_id.to_string(),
        attempt: non_negative_u32(row.attempt, "agent_run.attempt")?,
        status: parse_run_status(&row.status)?,
        waiting_reason: row
            .waiting_reason
            .as_deref()
            .map(parse_waiting_reason)
            .transpose()?,
        lease_owner: row.worker_id,
        lease_expires_at: row.lease_expires_at.map(|value| value.to_string()),
        budget: from_json(row.budget_json, "agent_run.budget_json")?,
        steps_used: json_u32(usage, "steps_used")?,
        model_calls_used: json_u32(usage, "model_calls_used")?,
        tool_calls_used: json_u32(usage, "tool_calls_used")?,
        tokens_used: json_u64(usage, "tokens_used")?,
        cost_micros: json_optional_u64(usage, "cost_micros")?,
        error: row.last_error,
        created_at: row.created_at.to_string(),
        updated_at: row.updated_at.to_string(),
    })
}

fn runtime_snapshot_required(usage_json: &Value, run_id: i64) -> Result<bool, AppError> {
    let usage = usage_json
        .as_object()
        .ok_or_else(|| AppError::InvalidState {
            current: format!("Agent run {run_id} usage_json 不是对象"),
            operation: "读取 Agent runtime snapshot admission".into(),
        })?;
    match usage.get("runtime_snapshot_required") {
        Some(Value::Bool(required)) => Ok(*required),
        Some(_) => Err(AppError::InvalidState {
            current: format!("Agent run {run_id} runtime_snapshot_required 不是 boolean"),
            operation: "读取 Agent runtime snapshot admission".into(),
        }),
        None => Err(AppError::InvalidState {
            current: format!("Agent run {run_id} 缺少 runtime_snapshot_required 标记"),
            operation: "读取 Agent runtime snapshot admission".into(),
        }),
    }
}

fn execution_view(row: execution::Model) -> Result<AgentExecutionView, AppError> {
    Ok(AgentExecutionView {
        id: row.id.to_string(),
        run_id: row.run_id.to_string(),
        sequence: non_negative_u64(row.sequence, "agent_execution.sequence")?,
        kind: parse_execution_kind(&row.execution_kind)?,
        tool_name: row.tool_name,
        status: parse_execution_status(&row.status)?,
        effect: row.effect,
        replay_policy: row
            .replay_policy
            .as_deref()
            .map(parse_replay_policy)
            .transpose()?,
        request_digest: row.arguments_sha256,
        resource_refs: resource_refs_from_value(
            row.resource_preconditions_json,
            "agent_execution.resource_preconditions_json",
        )?,
        result_refs: optional_json_vec(row.result_refs_json, "agent_execution.result_refs_json")?,
        result_summary: row.result_summary,
        error: row.error_message,
        created_at: row.created_at.to_string(),
        updated_at: row.updated_at.to_string(),
    })
}

fn delegation_view(row: delegation::Model) -> Result<AgentDelegationView, AppError> {
    Ok(AgentDelegationView {
        id: row.id.to_string(),
        parent_run_id: row.parent_run_id.to_string(),
        parent_execution_id: row.parent_execution_id.map(|id| id.to_string()),
        child_thread_id: row.child_thread_id.to_string(),
        child_turn_id: row.child_turn_id.map(|id| id.to_string()),
        child_run_id: row.child_run_id.map(|id| id.to_string()),
        capability: row.capability,
        task: row.objective,
        scope: from_json(row.scope_json, "agent_delegation.scope_json")?,
        status: parse_delegation_status(&row.status)?,
        success_criteria: from_json(row.success_criteria_json, "success_criteria_json")?,
        result_refs: optional_json_vec(row.result_refs_json, "delegation.result_refs_json")?,
        acceptance_summary: row.result_summary,
        acceptance_error: row.error_message,
        created_at: row.created_at.to_string(),
        updated_at: row.updated_at.to_string(),
    })
}

fn approval_view(row: approval::Model) -> Result<AgentApprovalView, AppError> {
    let operation = row
        .operation_json
        .as_object()
        .ok_or_else(|| AppError::InvalidState {
            current: format!("Agent approval {} operation_json 不是对象", row.id),
            operation: "读取 Agent approval".into(),
        })?;
    let status = parse_approval_status(&row.status)?;
    Ok(AgentApprovalView {
        id: row.id.to_string(),
        execution_id: row.execution_id.to_string(),
        status,
        command: operation
            .get("command")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| AppError::InvalidState {
                current: format!("Agent approval {} 缺少 command", row.id),
                operation: "读取 Agent approval".into(),
            })?
            .to_string(),
        args_digest: operation
            .get("args_digest")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or(Some(row.operation_digest)),
        resource_refs: operation
            .get("resource_refs")
            .cloned()
            .map(|value| from_json(value, "approval.resource_refs"))
            .transpose()?
            .unwrap_or_default(),
        resource_revision: operation
            .get("resource_revision")
            .and_then(Value::as_str)
            .map(str::to_string),
        expected_effect: operation
            .get("expected_effect")
            .and_then(Value::as_str)
            .map(str::to_string),
        expires_at: row.expires_at.map(|value| value.to_string()),
        decided_by_user_id: row.decided_by_user_id,
        decision: match status {
            AgentApprovalStatus::Approved => Some(AgentApprovalDecision::Approve),
            AgentApprovalStatus::Rejected => Some(AgentApprovalDecision::Reject),
            _ => None,
        },
        reason: row.decision_note,
        created_at: row.created_at.to_string(),
        decided_at: row.decided_at.map(|value| value.to_string()),
    })
}

fn inbox_view(row: inbox::Model) -> Result<AgentInboxItemView, AppError> {
    Ok(AgentInboxItemView {
        id: row.id.to_string(),
        thread_id: row.thread_id.to_string(),
        kind: parse_inbox_kind(&row.inbox_kind)?,
        status: parse_inbox_status(&row.status)?,
        delivery_mode: Some(parse_delivery_mode(&row.delivery_mode)?),
        message_id: row
            .payload_json
            .as_ref()
            .and_then(|payload| payload.get("message_id"))
            .and_then(Value::as_str)
            .map(str::to_string),
        content: Some(row.content),
        payload: row.payload_json,
        priority: i16::try_from(row.priority).map_err(|_| AppError::InvalidState {
            current: format!("Agent inbox {} priority 超出 i16", row.id),
            operation: "读取 Agent inbox".into(),
        })?,
        created_at: row.created_at.to_string(),
        consumed_at: row.consumed_at.map(|value| value.to_string()),
    })
}

fn memory_view(row: memory::Model) -> Result<AgentMemoryView, AppError> {
    let resource_refs = row
        .content_json
        .as_ref()
        .and_then(|value| value.get("resource_refs"))
        .cloned()
        .map(|value| from_json(value, "agent_memory.resource_refs"))
        .transpose()?
        .unwrap_or_default();
    Ok(AgentMemoryView {
        id: row.id.to_string(),
        scope_kind: parse_memory_scope_kind(&row.scope_kind)?,
        scope_id: row.scope_key,
        status: parse_memory_status(&row.status)?,
        memory_kind: row.memory_key,
        content: row.content,
        resource_refs,
        source_thread_id: row.thread_id.map(|id| id.to_string()),
        source_message_id: row.source_message_id.map(|id| id.to_string()),
        supersedes_id: row.supersedes_memory_id.map(|id| id.to_string()),
        created_at: row.created_at.to_string(),
        updated_at: row.updated_at.to_string(),
    })
}

fn event_view(row: event::Model) -> Result<AgentEvent, AppError> {
    let editor_request = row
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get("editor_request"))
        .filter(|value| !value.is_null())
        .cloned()
        .map(|value| from_json(value, "agent_event.editor_request"))
        .transpose()?;
    Ok(AgentEvent {
        protocol_version: AGENT_API_PROTOCOL_VERSION,
        thread_id: row.thread_id.to_string(),
        sequence: non_negative_u64(row.sequence, "agent_event.sequence")?,
        kind: row.kind,
        actor_kind: row.actor_kind,
        actor_id: row.actor_id,
        turn_id: row.turn_id.map(|id| id.to_string()),
        run_id: row.run_id.map(|id| id.to_string()),
        execution_id: row.execution_id.map(|id| id.to_string()),
        delegation_id: row.delegation_id.map(|id| id.to_string()),
        message_id: row.message_id.map(|id| id.to_string()),
        payload: event_payload(row.payload_json, row.public_text, row.status),
        editor_request,
        created_at: row.created_at.to_string(),
    })
}

fn event_payload(payload: Option<Value>, public_text: String, status: String) -> Option<Value> {
    match payload {
        Some(Value::Object(mut object)) => {
            if !public_text.is_empty() {
                object.entry("text").or_insert(Value::String(public_text));
            }
            object.entry("status").or_insert(Value::String(status));
            Some(Value::Object(object))
        }
        Some(payload) => Some(json!({
            "data": payload,
            "text": public_text,
            "status": status,
        })),
        None if public_text.is_empty() && status.is_empty() => None,
        None => Some(json!({ "text": public_text, "status": status })),
    }
}

fn from_json<T: DeserializeOwned>(value: Value, field: &str) -> Result<T, AppError> {
    serde_json::from_value(value).map_err(|error| AppError::InvalidState {
        current: format!("{field} 数据损坏: {error}"),
        operation: "读取 Agent 持久化状态".into(),
    })
}

fn optional_json_vec<T: DeserializeOwned>(
    value: Option<Value>,
    field: &str,
) -> Result<Vec<T>, AppError> {
    value
        .map(|value| from_json(value, field))
        .transpose()
        .map(Option::unwrap_or_default)
}

fn resource_refs_from_value(
    value: Option<Value>,
    field: &str,
) -> Result<Vec<AgentResourceRef>, AppError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    match value {
        Value::Array(_) => from_json(value, field),
        Value::Object(mut object) => object
            .remove("resource_refs")
            .map(|value| from_json(value, field))
            .transpose()
            .map(Option::unwrap_or_default),
        _ => Err(AppError::InvalidState {
            current: format!("{field} 既不是数组也不是对象"),
            operation: "读取 Agent 持久化状态".into(),
        }),
    }
}

fn non_negative_u64(value: i64, field: &str) -> Result<u64, AppError> {
    u64::try_from(value).map_err(|_| AppError::InvalidState {
        current: format!("{field} 为负数: {value}"),
        operation: "读取 Agent 持久化状态".into(),
    })
}

fn non_negative_u32(value: i32, field: &str) -> Result<u32, AppError> {
    u32::try_from(value).map_err(|_| AppError::InvalidState {
        current: format!("{field} 为负数: {value}"),
        operation: "读取 Agent 持久化状态".into(),
    })
}

fn json_u32(map: &serde_json::Map<String, Value>, key: &str) -> Result<u32, AppError> {
    u32::try_from(json_u64(map, key)?).map_err(|_| AppError::InvalidState {
        current: format!("Agent usage.{key} 超出 u32"),
        operation: "读取 Agent run".into(),
    })
}

fn json_u64(map: &serde_json::Map<String, Value>, key: &str) -> Result<u64, AppError> {
    match map.get(key) {
        None => Ok(0),
        Some(value) => value.as_u64().ok_or_else(|| AppError::InvalidState {
            current: format!("Agent usage.{key} 不是非负整数"),
            operation: "读取 Agent run".into(),
        }),
    }
}

fn json_optional_u64(
    map: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<u64>, AppError> {
    map.get(key)
        .map(|value| {
            value.as_u64().ok_or_else(|| AppError::InvalidState {
                current: format!("Agent usage.{key} 不是非负整数"),
                operation: "读取 Agent run".into(),
            })
        })
        .transpose()
}

macro_rules! parse_enum {
    ($name:ident, $ty:ty, {$($($raw:literal)|+ => $value:path),+ $(,)?}) => {
        fn $name(raw: &str) -> Result<$ty, AppError> {
            match raw {
                $($($raw)|+ => Ok($value),)+
                other => Err(AppError::InvalidState {
                    current: format!("未知 Agent 状态值: {other}"),
                    operation: "投影 Agent API DTO".into(),
                }),
            }
        }
    };
}

parse_enum!(parse_thread_kind, AgentThreadKind, {"main" => AgentThreadKind::Main, "subagent" => AgentThreadKind::Subagent});
parse_enum!(parse_message_role, AgentMessageRole, {"user" => AgentMessageRole::User, "assistant" => AgentMessageRole::Assistant, "system" => AgentMessageRole::System, "tool" => AgentMessageRole::Tool});
parse_enum!(parse_message_status, AgentMessageStatus, {"streaming" => AgentMessageStatus::Streaming, "complete" => AgentMessageStatus::Complete, "interrupted" => AgentMessageStatus::Interrupted});
parse_enum!(parse_turn_trigger, AgentTurnTrigger, {"user" => AgentTurnTrigger::User, "delegation" => AgentTurnTrigger::Delegation, "recovery" => AgentTurnTrigger::Recovery});
parse_enum!(parse_turn_status, AgentTurnStatus, {"queued" => AgentTurnStatus::Queued, "active" => AgentTurnStatus::Active, "completed" => AgentTurnStatus::Completed, "failed" => AgentTurnStatus::Failed, "cancelled" => AgentTurnStatus::Cancelled});
parse_enum!(parse_run_status, AgentRunStatus, {"queued" => AgentRunStatus::Queued, "running" => AgentRunStatus::Running, "waiting" => AgentRunStatus::Waiting, "pausing" => AgentRunStatus::Pausing, "paused" => AgentRunStatus::Paused, "cancelling" => AgentRunStatus::Cancelling, "cancelled" => AgentRunStatus::Cancelled, "completed" => AgentRunStatus::Completed, "failed" => AgentRunStatus::Failed, "interrupted" => AgentRunStatus::Interrupted, "recovery_required" => AgentRunStatus::RecoveryRequired});
parse_enum!(parse_waiting_reason, AgentWaitingReason, {"user" => AgentWaitingReason::User, "approval" => AgentWaitingReason::Approval, "child" => AgentWaitingReason::Child, "external" => AgentWaitingReason::External, "backoff" => AgentWaitingReason::Backoff, "budget" => AgentWaitingReason::Budget, "reconcile" => AgentWaitingReason::Reconcile});
parse_enum!(parse_execution_kind, AgentExecutionKind, {"model_call" => AgentExecutionKind::ModelCall, "tool_call" => AgentExecutionKind::ToolCall, "wait" => AgentExecutionKind::Wait, "delegation" => AgentExecutionKind::Delegation, "verification" => AgentExecutionKind::Verification});
parse_enum!(parse_execution_status, AgentExecutionStatus, {"prepared" => AgentExecutionStatus::Prepared, "awaiting_user" => AgentExecutionStatus::AwaitingUser, "dispatching" => AgentExecutionStatus::Dispatching, "running" => AgentExecutionStatus::Running, "succeeded" => AgentExecutionStatus::Succeeded, "failed" => AgentExecutionStatus::Failed, "rejected" => AgentExecutionStatus::Rejected, "cancelled" => AgentExecutionStatus::Cancelled, "outcome_unknown" => AgentExecutionStatus::OutcomeUnknown});
parse_enum!(parse_replay_policy, AgentToolReplayPolicy, {"safe" => AgentToolReplayPolicy::Safe, "idempotent" => AgentToolReplayPolicy::Idempotent, "reconcile_required" => AgentToolReplayPolicy::ReconcileRequired, "never" => AgentToolReplayPolicy::Never});
parse_enum!(parse_delegation_status, AgentDelegationStatus, {"pending" => AgentDelegationStatus::Pending, "running" => AgentDelegationStatus::Running, "awaiting_acceptance" => AgentDelegationStatus::AwaitingAcceptance, "revision_requested" => AgentDelegationStatus::RevisionRequested, "accepted" => AgentDelegationStatus::Accepted, "failed" => AgentDelegationStatus::Failed, "cancelled" => AgentDelegationStatus::Cancelled, "interrupted" => AgentDelegationStatus::Interrupted});
parse_enum!(parse_approval_status, AgentApprovalStatus, {"pending" => AgentApprovalStatus::Pending, "approved" => AgentApprovalStatus::Approved, "rejected" => AgentApprovalStatus::Rejected, "expired" => AgentApprovalStatus::Expired, "consumed" => AgentApprovalStatus::Consumed});
parse_enum!(parse_inbox_kind, AgentInboxKind, {"followup" => AgentInboxKind::Followup, "steer" => AgentInboxKind::Steer, "inject" => AgentInboxKind::Inject, "control" => AgentInboxKind::Control, "child_result" => AgentInboxKind::ChildResult});
parse_enum!(parse_inbox_status, AgentInboxStatus, {"pending" => AgentInboxStatus::Pending, "claimed" => AgentInboxStatus::Claimed, "consumed" => AgentInboxStatus::Consumed, "superseded" => AgentInboxStatus::Superseded, "cancelled" => AgentInboxStatus::Cancelled});
parse_enum!(parse_delivery_mode, AgentDeliveryMode, {"followup" => AgentDeliveryMode::Followup, "steer" => AgentDeliveryMode::Steer, "inject" => AgentDeliveryMode::Inject});
parse_enum!(parse_memory_scope_kind, AgentMemoryScopeKind, {"user" => AgentMemoryScopeKind::User, "project" => AgentMemoryScopeKind::Project, "thread" => AgentMemoryScopeKind::Thread});
parse_enum!(parse_memory_status, AgentMemoryStatus, {"proposed" => AgentMemoryStatus::Proposed, "active" => AgentMemoryStatus::Active, "superseded" => AgentMemoryStatus::Superseded, "retracted" => AgentMemoryStatus::Retracted});

fn agent_db_error_to_app(error: AgentDbError) -> AppError {
    match error {
        AgentDbError::BudgetExhausted(message) => AppError::InvalidState {
            current: format!("Agent 共享预算已耗尽: {message}"),
            operation: "预留执行额度".into(),
        },
        AgentDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        AgentDbError::NotFound(message) => AppError::NotFound(message),
        AgentDbError::Conflict(message) => AppError::InvalidState {
            current: message,
            operation: "执行 Agent 操作".into(),
        },
        AgentDbError::Database(error) => AppError::Database(error),
    }
}

fn collaboration_db_error_to_app(error: CollaborationDbError) -> AppError {
    match error {
        CollaborationDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        CollaborationDbError::NotFound(message) => AppError::NotFound(message),
        CollaborationDbError::Forbidden(message) => AppError::Forbidden(message),
        CollaborationDbError::InvalidState(message) => AppError::InvalidState {
            current: message,
            operation: "校验 Agent 资源访问权限".into(),
        },
        CollaborationDbError::Internal(message) => AppError::Internal(anyhow::anyhow!(message)),
        CollaborationDbError::Database(error) => AppError::Database(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_summary_validation_rejects_oversized_serialized_payload() {
        let summary = AgentContextSummary {
            current_goal: "goal".into(),
            constraints: vec!["x".repeat(4_000); 64],
            decisions: vec!["x".repeat(4_000); 64],
            facts_resources: vec!["x".repeat(4_000); 64],
            completed: vec!["x".repeat(4_000); 64],
            pending: vec!["x".repeat(4_000); 64],
            tool_state: vec!["x".repeat(4_000); 64],
            covered_through_sequence: 1,
        };
        assert!(summary.validate().is_err());
    }

    #[test]
    fn budget_validation_rejects_zero_limits_and_caps_child_budget() {
        let invalid = AgentBudget {
            max_children: Some(0),
            ..AgentBudget::default()
        };
        assert!(validate_agent_budget(&invalid).is_err());
        let invalid_duration = AgentBudget {
            max_duration_seconds: Some(i64::MAX as u64 + 1),
            ..AgentBudget::default()
        };
        assert!(validate_agent_budget(&invalid_duration).is_err());

        let parent = AgentBudget {
            max_steps: Some(10),
            max_model_calls: Some(8),
            max_tokens: Some(1_000),
            max_delegation_depth: Some(2),
            ..AgentBudget::default()
        };
        let child = AgentBudget {
            max_steps: Some(99),
            max_model_calls: Some(99),
            max_tokens: Some(9_999),
            max_delegation_depth: Some(9),
            ..AgentBudget::default()
        };
        let capped = child_budget_within_parent(
            child,
            &parent,
            &json!({
                "steps_used": 3,
                "model_calls_used": 2,
                "tokens_used": 250
            }),
        )
        .expect("child budget must be capped");
        assert_eq!(capped.max_steps, Some(7));
        assert_eq!(capped.max_model_calls, Some(6));
        assert_eq!(capped.max_tokens, Some(750));
        assert_eq!(capped.max_delegation_depth, Some(1));

        let exhausted = child_budget_within_parent(
            AgentBudget::default(),
            &AgentBudget {
                max_steps: Some(3),
                ..AgentBudget::default()
            },
            &json!({"steps_used": 3}),
        )
        .expect_err("an exhausted parent budget must not admit another child");
        assert!(exhausted.to_string().contains("max_steps"));
        let malformed = child_budget_within_parent(
            AgentBudget::default(),
            &AgentBudget::default(),
            &json!({"steps_used": "3"}),
        )
        .expect_err("malformed parent usage must not reset to zero");
        assert!(malformed.to_string().contains("steps_used"));
    }

    #[test]
    fn tool_search_excludes_hidden_commands_and_honors_limit() {
        let application = AgentApplication::new(DbHandle::new(std::sync::Arc::new(
            sea_orm::DatabaseConnection::Disconnected,
        )));
        let result = application
            .search_tools(SearchAgentToolsInput {
                query: None,
                domain: None,
                effect: None,
                capability: None,
                limit: Some(1),
            })
            .expect("search tools");
        assert_eq!(result.tools.len(), 1);
        assert!(result.total >= result.tools.len());
        assert_ne!(result.tools[0].exposure, AgentToolExposure::Hidden);

        let delegate = application
            .search_tools(SearchAgentToolsInput {
                query: Some("delegate".into()),
                domain: None,
                effect: None,
                capability: Some("main".into()),
                limit: None,
            })
            .expect("synthetic tools must be discoverable");
        assert_eq!(
            delegate
                .tools
                .iter()
                .map(|tool| tool.name.as_str())
                .collect::<Vec<_>>(),
            ["delegate", "review_agent_delegation"]
        );

        let generic = application
            .search_tools(SearchAgentToolsInput {
                query: Some("create_project".into()),
                domain: None,
                effect: None,
                capability: Some("general".into()),
                limit: None,
            })
            .expect("generic capability should search the full manifest");
        assert_eq!(
            generic
                .tools
                .iter()
                .map(|tool| tool.name.as_str())
                .collect::<Vec<_>>(),
            ["create_project"]
        );
    }

    #[test]
    fn inbox_claimed_status_remains_distinct_from_pending() {
        assert_eq!(
            parse_inbox_status("claimed").expect("claimed is a live lease state"),
            AgentInboxStatus::Claimed
        );
        assert!(parse_inbox_status("legacy").is_err());
    }

    #[test]
    fn remote_new_run_requires_runtime_snapshot_but_local_run_does_not() {
        let error = admit_new_run_runtime_snapshot(false, true)
            .expect_err("remote new run without snapshot must be rejected");
        assert!(error.to_string().contains("runtime_config_snapshot"));
        admit_new_run_runtime_snapshot(true, true).expect("remote snapshot must be admitted");
        admit_new_run_runtime_snapshot(false, false).expect("local runtime may omit snapshot");
    }

    #[test]
    fn canonical_subagent_key_matches_runtime_domain_binding() {
        let chapter = canonical_subagent_binding_key(
            "content_optimization",
            &AgentResourceRef {
                kind: "chapter".into(),
                id: "11".into(),
                label: None,
                revision: None,
                hash: None,
            },
        )
        .expect("chapter binding");
        assert_eq!(chapter, "subagent:content_optimization:chapter:11");

        let project = canonical_subagent_binding_key(
            "canvas",
            &AgentResourceRef {
                kind: "project".into(),
                id: "7".into(),
                label: None,
                revision: None,
                hash: None,
            },
        )
        .expect("project binding");
        assert_eq!(project, "subagent:canvas:project:7");
    }

    #[test]
    fn generic_subagent_tool_scope_is_discovery_open() {
        let mut scope = AgentScope::default();
        apply_subagent_tool_policy("research", &mut scope)
            .expect("generic capability should remain discovery-open");
        assert!(scope.allowed_tools.is_empty());

        scope.allowed_tools = vec!["create_project".into()];
        apply_subagent_tool_policy("research", &mut scope)
            .expect("explicit generic manifest tool should be valid");
        scope.allowed_tools = vec!["create_llm_config".into()];
        assert!(apply_subagent_tool_policy("research", &mut scope).is_err());
    }

    #[test]
    fn generic_children_keep_nested_delegation_open_but_domains_do_not() {
        let mut generic = AgentScope::default();
        apply_subagent_capability_policy("research", &mut generic)
            .expect("generic capability should remain open");
        assert!(generic.allowed_capabilities.is_empty());

        let mut optimize = AgentScope::default();
        apply_subagent_capability_policy("content_optimization", &mut optimize)
            .expect("domain capability should receive its own fence");
        assert_eq!(optimize.allowed_capabilities, ["content_optimization"]);

        let parent = AgentScope {
            allowed_capabilities: vec!["research".into()],
            allowed_tools: vec!["list_projects".into()],
            ..AgentScope::default()
        };
        inherit_subagent_allowlists(&parent, &mut generic)
            .expect("explicit parent allowlists should be inherited");
        assert_eq!(generic.allowed_capabilities, ["research"]);
        assert_eq!(generic.allowed_tools, ["list_projects"]);
    }

    #[tokio::test]
    async fn remote_admission_rejects_before_persisting_turn_or_run() {
        let database =
            std::sync::Arc::new(a4n_migration::test_support::connect_test_database().await);
        let application = AgentApplication::new(DbHandle::new(database.clone()));
        let owner_user_id = 1;
        let thread = agent::create_thread(
            database.as_ref(),
            AgentThreadCreateInput {
                owner_user_id,
                thread_kind: "main".into(),
                profile_id: "main".into(),
                title: Some("snapshot admission test".into()),
                project_id: None,
                resource_type: None,
                resource_id: None,
                canonical_binding_key: Some(format!(
                    "agent-snapshot-admission:{}",
                    uuid::Uuid::new_v4()
                )),
                scope_json: serde_json::to_value(AgentScope::default()).unwrap(),
                default_model_config_id: None,
                is_default: false,
            },
        )
        .await
        .expect("thread must be created");

        let error = application
            .send_message(
                owner_user_id,
                SendAgentMessageInput {
                    thread_id: thread.id.to_string(),
                    client_message_id: uuid::Uuid::new_v4().to_string(),
                    content: "需要远程快照".into(),
                    ..SendAgentMessageInput::default()
                },
                false,
                true,
            )
            .await
            .expect_err("missing remote snapshot must be rejected");
        assert!(error.to_string().contains("runtime_config_snapshot"));
        assert!(
            agent::runs_by_thread(database.as_ref(), thread.id, SNAPSHOT_LIMIT)
                .await
                .expect("runs query")
                .is_empty()
        );
        assert!(
            agent::messages_by_thread(database.as_ref(), thread.id, SNAPSHOT_LIMIT)
                .await
                .expect("messages query")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn user_spawn_honors_parent_delegation_budget() {
        let database =
            std::sync::Arc::new(a4n_migration::test_support::connect_test_database().await);
        let application = AgentApplication::new(DbHandle::new(database.clone()));
        let parent_thread = agent::create_thread(
            database.as_ref(),
            AgentThreadCreateInput {
                owner_user_id: 1,
                thread_kind: "main".into(),
                profile_id: "main".into(),
                title: Some("spawn budget test".into()),
                project_id: None,
                resource_type: None,
                resource_id: None,
                canonical_binding_key: Some(format!("agent-spawn-budget:{}", uuid::Uuid::new_v4())),
                scope_json: serde_json::to_value(AgentScope::default()).unwrap(),
                default_model_config_id: None,
                is_default: false,
            },
        )
        .await
        .expect("parent thread must be created");
        let client_message_id = uuid::Uuid::new_v4().to_string();
        let (_, parent_run, _) = agent::create_turn_with_run(
            database.as_ref(),
            AgentTurnInput {
                thread_id: parent_thread.id,
                client_message_id: client_message_id.clone(),
                message: AgentMessageInput {
                    thread_id: parent_thread.id,
                    turn_id: None,
                    run_id: None,
                    role: "user".into(),
                    sender_kind: "user".into(),
                    sender_thread_id: None,
                    client_message_id: Some(client_message_id),
                    content: "创建一个子 Agent".into(),
                    status: "complete".into(),
                    payload_json: None,
                },
                execution_mode: "autonomous".into(),
                model_override_json: None,
                budget_json: serde_json::to_value(AgentBudget {
                    max_children: Some(1),
                    max_steps: Some(10),
                    ..AgentBudget::default()
                })
                .unwrap(),
                runtime_snapshot_required: false,
                trigger: "user".into(),
                goal: Some("创建一个子 Agent".into()),
            },
        )
        .await
        .expect("parent run must be created");

        let spawn = |task: &str| SpawnAgentInput {
            parent_run_id: parent_run.id.to_string(),
            capability: "general".into(),
            task: task.into(),
            title: None,
            bound_resource: None,
            scope: AgentScope::default(),
            success_criteria: Vec::new(),
            budget: Some(AgentBudget {
                max_steps: Some(99),
                ..AgentBudget::default()
            }),
            runtime_config_snapshot: None,
        };
        application
            .spawn_agent(1, spawn("第一个任务"))
            .await
            .expect("first child must be admitted");
        let error = application
            .spawn_agent(1, spawn("第二个任务"))
            .await
            .expect_err("max_children must apply to user spawn");
        assert!(error.to_string().contains("max_children"));
    }
}
