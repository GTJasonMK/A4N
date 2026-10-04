use super::super::{
    AgentApplication, AgentWorkerRun, WorkerDelegationInput, WorkerDelegationResult,
};
use super::*;
use a4n_agent_runtime_contract::AgentWorkerDelegationApplicationPort;
use a4n_api_contract::agent::AgentScope;
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentWorkerDelegationStartRequest, AgentWorkerDelegationStartSnapshotWire,
};

impl AgentApplication {
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
        let child_thread = agent::thread_by_id(self.executor(), input.child_thread_id)
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
        let parent_turn = agent::turn_by_id(self.executor(), claimed.run.turn_id)
            .await
            .map_err(agent_db_error_to_app)?
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
            self.executor(),
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
}

#[async_trait::async_trait]
impl AgentWorkerDelegationApplicationPort for AgentApplication {
    async fn create_child_run_request(
        &self,
        request: &AgentWorkerDelegationStartRequest,
    ) -> Result<AgentWorkerDelegationStartSnapshotWire, AppError> {
        AgentApplication::create_child_run_request(self, request).await
    }

    fn inherit_subagent_allowlists(
        &self,
        parent: &AgentScope,
        child: &mut AgentScope,
    ) -> Result<(), AppError> {
        super::super::inherit_subagent_allowlists(parent, child)
    }

    async fn create_child_run(
        &self,
        claimed: &AgentWorkerRun,
        input: WorkerDelegationInput,
    ) -> Result<WorkerDelegationResult, AppError> {
        AgentApplication::create_child_run(self, claimed, input).await
    }
}
