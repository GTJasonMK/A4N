use super::*;

pub fn agent_worker_run_from_wire(wire: AgentWorkerRunWire) -> Result<AgentWorkerRun, AppError> {
    AgentWorkerRun::from_wire(wire)
}

pub(crate) fn worker_request_for_claimed_run(
    claimed: &AgentWorkerRun,
    expected_worker_id: &str,
    runtime_snapshot: Option<a4n_api_contract::runtime_config::RuntimeConfigSnapshot>,
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

pub(crate) fn validate_claimed_worker_request(
    claimed: &AgentWorkerRun,
    request: &WorkerRequest<AgentRunPayload>,
) -> Result<(), AppError> {
    request.validate_agent_run().map_err(|error| {
        AppError::InvalidArgument(format!("Agent worker payload 无效: {error}"))
    })?;
    let runtime_snapshot = request
        .payload
        .runtime_config_snapshot
        .clone()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| AppError::InvalidState {
            current: format!("Agent worker request runtime snapshot 无效: {error}"),
            operation: "校验 Agent worker request".into(),
        })?;
    let expected =
        worker_request_for_claimed_run(claimed, &request.lease.worker_id, runtime_snapshot)?;
    if request != &expected {
        return Err(AppError::Forbidden(
            "Agent worker request 与当前 Run lease 或 payload 不匹配".into(),
        ));
    }
    Ok(())
}
