//! worker execution 与 child run。

use super::super::WorkerUsage;
use super::super::{
    AgentApplication, AgentExecutionSnapshot, WorkerExecutionInput, WorkerExecutionUpdate,
};
use super::*;
use a4n_agent_runtime_contract::AgentWorkerExecutionApplicationPort;
use a4n_agent_runtime_contract::AgentWorkerUsageApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerUsageRequest;
use a4n_worker_contract::{
    AgentWorkerExecutionCreateRequest, AgentWorkerExecutionLookupRequest,
    AgentWorkerExecutionUpdateRequest,
};

impl AgentApplication {
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
            Ok(execution) => Ok(Some(execution_snapshot(execution))),
            Err(AgentDbError::BudgetExhausted(_)) => Ok(None),
            Err(error) => Err(agent_db_error_to_app(error)),
        }
    }

    pub(crate) async fn worker_execution_by_key_request(
        &self,
        request: &AgentWorkerExecutionLookupRequest,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        self.worker_execution_by_key(request.run_id, &request.execution_key)
            .await
    }

    pub(crate) async fn create_prepared_execution_request(
        &self,
        request: &AgentWorkerExecutionCreateRequest,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        self.create_prepared_execution(WorkerExecutionInput {
            run_id: request.run_id,
            lease_token: request.lease.fence_token.clone(),
            execution_key: request.execution_key.clone(),
            execution_kind: request.execution_kind.clone(),
            tool_name: request.tool_name.clone(),
            effect: request.effect.clone(),
            replay_policy: request.replay_policy.clone(),
            arguments_json: request.arguments_json.clone(),
            resource_preconditions_json: request.resource_preconditions_json.clone(),
        })
        .await
    }

    pub(crate) async fn update_worker_execution_request(
        &self,
        request: &AgentWorkerExecutionUpdateRequest,
    ) -> Result<AgentExecutionSnapshot, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        self.update_worker_execution(WorkerExecutionUpdate {
            run_id: request.run_id,
            lease_token: request.lease.fence_token.clone(),
            execution_id: request.execution_id,
            status: request.status.clone(),
            result_json: request.result_json.clone(),
            result_summary: request.result_summary.clone(),
            result_refs_json: request.result_refs_json.clone(),
            error_message: request.error_message.clone(),
        })
        .await
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
        let (project_id, chapter_id, chapter_number) = self
            .verify_thread_scope(
                claimed.thread.owner_user_id,
                &child_scope,
                child_resource.as_ref(),
            )
            .await?;
        child_scope.project_id = child_scope.project_id.or(project_id);
        child_scope.chapter_id = child_scope.chapter_id.or(chapter_id);
        child_scope.chapter_number = chapter_number;
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
        if runtime_snapshot_required && input.runtime_config_snapshot.is_none() {
            return Err(AppError::InvalidState {
                current: "Agent child run 缺少 runtime_config_snapshot".into(),
                operation: "创建 Agent child delegation".into(),
            });
        }
        let scope_json = serde_json::to_value(&child_scope)?;
        let client_message_id = format!("delegation:{}:{delegation_key}", claimed.run.id);
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
            delegation: delegation_snapshot(delegation),
            child_turn: turn_snapshot(child_turn),
            child_run: run_snapshot(child_run),
            child_message: message_snapshot(child_message),
        })
    }

    pub(crate) async fn create_child_run_request(
        &self,
        request: &AgentWorkerDelegationStartRequest,
    ) -> Result<AgentWorkerDelegationStartSnapshotWire, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let claimed = self
            .worker_run_for_lease_request(&AgentWorkerRunLeaseRequest {
                run_id: request.run_id,
                lease: request.lease.clone(),
            })
            .await?;
        self.create_child_run(
            &claimed,
            WorkerDelegationInput {
                capability: request.capability.clone(),
                objective: request.objective.clone(),
                delegation_key: request.delegation_key.clone(),
                child_thread_id: request.child_thread_id,
                scope: request.scope.clone(),
                success_criteria: request.success_criteria.clone(),
                budget_json: request.budget_json.clone(),
                parent_execution_id: request.parent_execution_id,
                runtime_config_snapshot: request.runtime_config_snapshot.clone(),
            },
        )
        .await
        .map(|result| AgentWorkerDelegationStartSnapshotWire {
            delegation: result.delegation.into(),
            child_turn: result.child_turn.into(),
            child_run: result.child_run.into(),
            child_message: result.child_message.into(),
        })
    }

    pub(super) async fn complete_child_run_fields(
        &self,
        fields: RunLeaseFields<'_>,
        source_execution_id: i64,
        content: String,
        result_summary: String,
        result_refs_json: Option<Value>,
        observed_input_message_id: i64,
    ) -> Result<a4n_worker_contract::AgentWorkerCompletionSnapshotWire, AppError> {
        super::completion::completion_receipt(
            agent::complete_child_delegation(
                self.connection(),
                a4n_db::agent::AgentChildCompletionInput {
                    child_run_id: fields.run_id,
                    child_lease_token: fields.lease_token.to_string(),
                    final_message: AgentMessageInput {
                        thread_id: fields.thread_id,
                        turn_id: Some(fields.turn_id),
                        run_id: Some(fields.run_id),
                        role: "assistant".into(),
                        sender_kind: "agent".into(),
                        sender_thread_id: None,
                        client_message_id: Some(format!("agent-final:{}", fields.run_id)),
                        content,
                        status: "complete".into(),
                        payload_json: Some(json!({"completion_execution_id": source_execution_id})),
                    },
                    result_refs_json,
                    result_summary,
                    observed_input_message_id,
                },
            )
            .await,
        )
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
        .map(execution_snapshot)
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

    pub(crate) async fn update_run_usage_request(
        &self,
        request: &AgentWorkerUsageRequest,
    ) -> Result<(), AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        agent::update_run_usage(
            self.connection(),
            request.run_id,
            &request.lease.fence_token,
            request.usage_json.clone(),
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }
}

#[async_trait::async_trait]
impl AgentWorkerExecutionApplicationPort for AgentApplication {
    async fn worker_execution_by_key(
        &self,
        run_id: i64,
        key: &str,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        AgentApplication::worker_execution_by_key(self, run_id, key).await
    }

    async fn create_prepared_execution(
        &self,
        input: WorkerExecutionInput,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        AgentApplication::create_prepared_execution(self, input).await
    }

    async fn update_worker_execution(
        &self,
        input: WorkerExecutionUpdate,
    ) -> Result<AgentExecutionSnapshot, AppError> {
        AgentApplication::update_worker_execution(self, input).await
    }

    async fn worker_execution_by_key_request(
        &self,
        request: &AgentWorkerExecutionLookupRequest,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        AgentApplication::worker_execution_by_key_request(self, request).await
    }

    async fn create_prepared_execution_request(
        &self,
        request: &AgentWorkerExecutionCreateRequest,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        AgentApplication::create_prepared_execution_request(self, request).await
    }

    async fn update_worker_execution_request(
        &self,
        request: &AgentWorkerExecutionUpdateRequest,
    ) -> Result<AgentExecutionSnapshot, AppError> {
        AgentApplication::update_worker_execution_request(self, request).await
    }
}

#[async_trait::async_trait]
impl AgentWorkerUsageApplicationPort for AgentApplication {
    async fn update_run_usage(&self, input: WorkerUsage) -> Result<(), AppError> {
        AgentApplication::update_run_usage(self, input)
            .await
            .map(|_| ())
    }

    async fn update_run_usage_request(
        &self,
        request: &AgentWorkerUsageRequest,
    ) -> Result<(), AppError> {
        AgentApplication::update_run_usage_request(self, request).await
    }
}
