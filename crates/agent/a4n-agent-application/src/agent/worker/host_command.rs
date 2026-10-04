use super::super::{AgentApplication, AgentScope};
use super::*;
use a4n_agent_runtime_contract::AgentWorkerHostCommandApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::{AgentHostCommandPayload, WorkerEnvelope};

impl AgentApplication {
    /// Authorizes a dynamic Host command returned by an external Agent worker.
    /// Every identity and fencing field is checked against the durable Run
    /// before the control plane executes the command on the user's behalf.
    pub async fn authorize_worker_host_command(
        &self,
        request: &WorkerEnvelope<AgentHostCommandPayload>,
    ) -> Result<i64, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(format!("worker request 无效: {error}")))?;
        request.payload.validate().map_err(|error| {
            AppError::InvalidArgument(format!("Agent Host command payload 无效: {error}"))
        })?;
        if request.operation_id != a4n_worker_contract::AGENT_HOST_COMMAND_OPERATION {
            return Err(AppError::InvalidArgument(format!(
                "不支持的 Agent worker operation: {}",
                request.operation_id
            )));
        }
        let run = agent::run_by_id(self.executor(), request.payload.run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| {
                AppError::NotFound(format!("Agent run 不存在: {}", request.payload.run_id))
            })?;
        agent::validate_live_run_lease_record(
            &run,
            &request.lease.fence_token,
            chrono::Utc::now().naive_utc(),
        )
        .map_err(|error| match error {
            AgentDbError::Conflict(message) => AppError::Forbidden(message),
            other => agent_db_error_to_app(other),
        })?;
        if run.worker_id.as_deref() != Some(request.lease.worker_id.as_str())
            || u64::try_from(run.attempt).ok() != Some(request.lease.epoch)
        {
            return Err(AppError::Forbidden(
                "Agent worker request 与当前 Run lease 不一致".into(),
            ));
        }
        // `expires_at` is an observation captured when the request was built.
        // Renewal legitimately changes it while the fence token and epoch stay
        // stable; validate liveness above against the database as authority.
        let thread = agent::thread_by_id(self.executor(), run.thread_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent thread 不存在: {}", run.thread_id)))?;
        if let Some(execution_id) = request.payload.execution_id {
            let execution = agent::execution_by_id(self.executor(), execution_id)
                .await
                .map_err(agent_db_error_to_app)?
                .ok_or_else(|| {
                    AppError::NotFound(format!("Agent execution 不存在: {execution_id}"))
                })?;
            if execution.run_id != run.id {
                return Err(AppError::Forbidden(
                    "Agent Host command 的 execution 不属于当前 Run".into(),
                ));
            }
        }
        let mut stored_scope: AgentScope =
            from_json(thread.scope_json.clone(), "agent_thread.scope_json")?;
        self.hydrate_scope_chapter_number(&mut stored_scope).await?;
        let expected_scope = effective_agent_scope(&thread.profile_id, stored_scope)?;
        if !agent_scopes_match_except_chapter_number(&request.payload.scope, &expected_scope) {
            return Err(AppError::Forbidden(
                "Agent Host command 的 scope 与当前 Run 不一致".into(),
            ));
        }
        let user_id = request
            .principal
            .subject
            .parse::<i64>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| AppError::Forbidden("Agent worker principal 无效".into()))?;
        if request.principal.kind != a4n_worker_contract::WorkerPrincipalKind::User
            || thread.owner_user_id != user_id
            || request.project_id != thread.project_id
            || request.target_revision
                != Some(
                    u64::try_from(thread.revision).map_err(|_| AppError::InvalidState {
                        current: format!("Agent thread {} 的 revision 无效", thread.id),
                        operation: "校验 Agent worker request".into(),
                    })?,
                )
        {
            return Err(AppError::Forbidden(
                "Agent worker request 的 principal 或资源版本不匹配".into(),
            ));
        }
        Ok(user_id)
    }
}

#[async_trait::async_trait]
impl AgentWorkerHostCommandApplicationPort for AgentApplication {
    async fn authorize_worker_host_command(
        &self,
        request: &WorkerEnvelope<AgentHostCommandPayload>,
    ) -> Result<i64, AppError> {
        self.authorize_worker_host_command(request).await
    }
}
