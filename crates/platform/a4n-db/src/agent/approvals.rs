use super::*;
use sea_orm::ConnectionTrait;

pub(super) fn approval_authorizes_execution(
    status: &str,
    replay_policy: Option<&str>,
    execution_status: &str,
) -> bool {
    status == "approved"
        || (status == "consumed"
            && replay_policy == Some("idempotent")
            && matches!(execution_status, "dispatching" | "running"))
}

pub async fn approved_execution<C: ConnectionTrait>(
    db: &C,
    thread_id: i64,
    execution_id: i64,
) -> Result<Option<AgentApprovalRecord>, AgentDbError> {
    let Some(grant) = approval::Entity::find()
        .filter(approval::Column::ThreadId.eq(thread_id))
        .filter(approval::Column::ExecutionId.eq(execution_id))
        .one(db)
        .await?
    else {
        return Ok(None);
    };
    let execution = execution::Entity::find_by_id(execution_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent execution 不存在: {execution_id}")))?;
    if execution.run_id != grant.run_id {
        return Err(AgentDbError::Conflict("审批与执行的 run 不一致".into()));
    }
    Ok(approval_authorizes_execution(
        &grant.status,
        execution.replay_policy.as_deref(),
        &execution.status,
    )
    .then_some(grant.into()))
}

#[cfg(test)]
mod replay_tests {
    use super::approval_authorizes_execution;
    #[test]
    fn consumption_only_authorizes_recovery_of_the_same_idempotent_execution() {
        assert!(approval_authorizes_execution(
            "approved",
            Some("never"),
            "awaiting_user"
        ));
        assert!(approval_authorizes_execution(
            "consumed",
            Some("idempotent"),
            "running"
        ));
        assert!(!approval_authorizes_execution(
            "consumed",
            Some("never"),
            "running"
        ));
        assert!(!approval_authorizes_execution(
            "consumed",
            Some("idempotent"),
            "prepared"
        ));
        assert!(!approval_authorizes_execution(
            "rejected",
            Some("idempotent"),
            "running"
        ));
    }
}

pub(crate) async fn create_approval<C>(
    db: &C,
    input: AgentApprovalInput,
) -> Result<approval::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    let timestamp = now();
    approval::ActiveModel {
        thread_id: Set(input.thread_id),
        run_id: Set(input.run_id),
        execution_id: Set(input.execution_id),
        operation_digest: Set(input.operation_digest),
        status: Set("pending".to_string()),
        operation_json: Set(input.operation_json),
        resource_preconditions_json: Set(input.resource_preconditions_json),
        decided_by_user_id: Set(None),
        decision_note: Set(None),
        expires_at: Set(input.expires_at),
        created_at: Set(timestamp),
        decided_at: Set(None),
        consumed_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AgentDbError::from)
}

/// Atomically requests user approval for a prepared write execution.
///
/// Registering the approval, moving the Execution to `awaiting_user`, and
/// releasing the Run lease into `waiting(approval)` are one durable decision.
/// This prevents an approval response from arriving between separate commits
/// and being lost before the worker starts waiting.
pub async fn request_approval_for_run_lease(
    db: &impl crate::DbExecutor,
    lease_token: &str,
    input: AgentApprovalInput,
) -> Result<(AgentApprovalRecord, AgentExecutionRecord, AgentRunRecord), AgentDbError> {
    if input.operation_digest.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "Agent approval operation_digest 不能为空".into(),
        ));
    }
    let identity = run::Entity::find_by_id(input.run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {}", input.run_id)))?;
    if identity.thread_id != input.thread_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent approval 与 run thread 不一致: {}",
            input.run_id
        )));
    }
    let tx = db.begin().await?;
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    let run_row = run::Entity::find_by_id(input.run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {}", input.run_id)))?;
    let timestamp = now();
    validate_live_run_lease(&run_row, lease_token, timestamp)?;
    let execution_row = execution::Entity::find_by_id(input.execution_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent execution 不存在: {}", input.execution_id))
        })?;
    if execution_row.run_id != run_row.id {
        return Err(AgentDbError::Conflict(format!(
            "Agent approval execution 不属于 run {}: {}",
            run_row.id, execution_row.id
        )));
    }
    if execution_row.status != "prepared" {
        return Err(AgentDbError::Conflict(format!(
            "Agent execution 不能请求审批: {} ({})",
            execution_row.id, execution_row.status
        )));
    }
    if approval::Entity::find()
        .filter(approval::Column::ExecutionId.eq(execution_row.id))
        .lock_exclusive()
        .one(&tx)
        .await?
        .is_some()
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent execution 已存在 approval: {}",
            execution_row.id
        )));
    }

    let approval_row = create_approval(&tx, input).await?;
    let mut execution_active: execution::ActiveModel = execution_row.into();
    execution_active.status = Set("awaiting_user".to_string());
    execution_active.updated_at = Set(timestamp);
    let execution_row = execution_active.update(&tx).await?;

    let mut run_active: run::ActiveModel = run_row.into();
    run_active.status = Set("waiting".to_string());
    run_active.waiting_reason = Set(Some("approval".to_string()));
    run_active.worker_id = Set(None);
    run_active.lease_token = Set(None);
    run_active.lease_expires_at = Set(None);
    run_active.updated_at = Set(timestamp);
    let run_row = run_active.update(&tx).await?;
    append_run_event_on(
        &tx,
        &run_row,
        "approval_requested",
        "pending",
        "Agent 操作等待用户审批",
        Some(execution_row.id),
        None,
        None,
        Some(json!({ "approval_id": approval_row.id.to_string() })),
    )
    .await?;
    let _ = refresh_thread_status_on(&tx, run_row.thread_id).await?;
    tx.commit().await?;
    Ok((approval_row.into(), execution_row.into(), run_row.into()))
}

/// Resolves an approval while holding the related Run/Execution locks.
/// Approved operations are left in `awaiting_user` for the worker to dispatch;
/// rejected operations are terminally rejected.  A waiting parent Run is
/// requeued in either case so it can make the next model decision.
pub async fn resolve_approval_for_run(
    db: &impl crate::DbExecutor,
    approval_id: i64,
    user_id: i64,
    approved: bool,
    reason: Option<String>,
) -> Result<AgentApprovalRecord, AgentDbError> {
    if user_id <= 0 {
        return Err(AgentDbError::InvalidArgument("user_id 无效".into()));
    }
    let identity = approval::Entity::find_by_id(approval_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent approval 不存在: {approval_id}")))?;
    let run_identity = run::Entity::find_by_id(identity.run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {}", identity.run_id)))?;
    if identity.thread_id != run_identity.thread_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent approval 关联记录不一致: {approval_id}"
        )));
    }
    if identity
        .expires_at
        .is_some_and(|expires_at| expires_at <= now())
    {
        return expire_pending_approval(db, approval_id)
            .await?
            .map(Into::into)
            .ok_or_else(|| {
                AgentDbError::Conflict(format!("Agent approval 已由其他请求处理: {approval_id}"))
            });
    }
    let tx = db.begin().await?;
    thread::Entity::find_by_id(run_identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", run_identity.thread_id))
        })?;
    let run_row = run::Entity::find_by_id(identity.run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {}", identity.run_id)))?;
    let execution_row = execution::Entity::find_by_id(identity.execution_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent execution 不存在: {}", identity.execution_id))
        })?;
    let approval_row = approval::Entity::find_by_id(approval_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent approval 不存在: {approval_id}")))?;
    if approval_row.status != "pending" {
        return Err(AgentDbError::Conflict(format!(
            "Agent approval 已处理: {approval_id}"
        )));
    }
    let timestamp = now();
    if approval_row
        .expires_at
        .is_some_and(|expires_at| expires_at <= timestamp)
    {
        tx.rollback().await?;
        return expire_pending_approval(db, approval_id)
            .await?
            .map(Into::into)
            .ok_or_else(|| {
                AgentDbError::Conflict(format!("Agent approval 已由其他请求处理: {approval_id}"))
            });
    }
    if execution_row.run_id != run_row.id || approval_row.thread_id != run_row.thread_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent approval 关联记录不一致: {approval_id}"
        )));
    }
    if !run_row.is_active {
        return Err(AgentDbError::Conflict(format!(
            "Agent approval 所属 run 已终止: {}",
            run_row.id
        )));
    }
    if execution_row.status != "awaiting_user" {
        return Err(AgentDbError::Conflict(format!(
            "Agent approval execution 不在 awaiting_user: {} ({})",
            execution_row.id, execution_row.status
        )));
    }
    let mut approval_active: approval::ActiveModel = approval_row.into();
    approval_active.status = Set(if approved { "approved" } else { "rejected" }.to_string());
    approval_active.decided_by_user_id = Set(Some(user_id));
    approval_active.decision_note = Set(reason);
    approval_active.decided_at = Set(Some(timestamp));
    let approval_row = approval_active.update(&tx).await?;

    if !approved {
        let mut execution_active: execution::ActiveModel = execution_row.into();
        execution_active.status = Set("rejected".to_string());
        execution_active.error_message = Set(Some("用户拒绝 Agent 操作".to_string()));
        execution_active.updated_at = Set(timestamp);
        execution_active.completed_at = Set(Some(timestamp));
        execution_active.update(&tx).await?;
    }
    let wake_parent =
        run_row.status == "waiting" && run_row.waiting_reason.as_deref() == Some("approval");
    let mut run_active: run::ActiveModel = run_row.into();
    if wake_parent {
        run_active.status = Set("queued".to_string());
        run_active.waiting_reason = Set(None);
        run_active.worker_id = Set(None);
        run_active.lease_token = Set(None);
        run_active.lease_expires_at = Set(None);
        run_active.next_attempt_at = Set(timestamp);
        run_active.updated_at = Set(timestamp);
        let run_row = run_active.update(&tx).await?;
        let _ = refresh_thread_status_on(&tx, run_row.thread_id).await?;
    }
    append_event_on(
        &tx,
        AgentEventInput {
            thread_id: run_identity.thread_id,
            turn_id: Some(run_identity.turn_id),
            run_id: Some(run_identity.id),
            execution_id: Some(identity.execution_id),
            delegation_id: None,
            message_id: None,
            actor_kind: "user".into(),
            actor_id: Some(user_id.to_string()),
            kind: "approval_resolved".into(),
            status: approval_row.status.clone(),
            public_text: if approved {
                "用户已批准 Agent 操作".into()
            } else {
                "用户已拒绝 Agent 操作".into()
            },
            payload_json: Some(json!({
                "approval_id": approval_row.id.to_string(),
                "reason": approval_row.decision_note.clone(),
            })),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(approval_row.into())
}

/// Expires pending approvals whose deadline has passed.  Expiry is processed
/// as a durable workflow transition so a run cannot remain indefinitely in
/// `waiting(approval)` when nobody opens the approval dialog.
pub async fn expire_pending_approvals(
    db: &impl crate::DbExecutor,
    limit: u64,
) -> Result<Vec<AgentApprovalRecord>, AgentDbError> {
    if limit == 0 {
        return Err(AgentDbError::InvalidArgument(
            "approval expiry limit 必须大于 0".into(),
        ));
    }
    let ids = approval::Entity::find()
        .select_only()
        .column(approval::Column::Id)
        .filter(approval::Column::Status.eq("pending"))
        .filter(approval::Column::ExpiresAt.is_not_null())
        .filter(approval::Column::ExpiresAt.lte(now()))
        .order_by_asc(approval::Column::ExpiresAt)
        .order_by_asc(approval::Column::Id)
        .limit(limit)
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let mut expired = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(row) = expire_pending_approval(db, id).await? {
            expired.push(row);
        }
    }
    Ok(expired.into_iter().map(AgentApprovalRecord::from).collect())
}

/// Attempts one expiry transition.  `None` means another resolver won the
/// race or the deadline was extended before the row was locked.
pub(crate) async fn expire_pending_approval(
    db: &impl crate::DbExecutor,
    approval_id: i64,
) -> Result<Option<approval::Model>, AgentDbError> {
    let identity = approval::Entity::find_by_id(approval_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent approval 不存在: {approval_id}")))?;
    let run_identity = run::Entity::find_by_id(identity.run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {}", identity.run_id)))?;
    if identity.thread_id != run_identity.thread_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent approval 关联记录不一致: {approval_id}"
        )));
    }
    let tx = db.begin().await?;
    thread::Entity::find_by_id(run_identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", run_identity.thread_id))
        })?;
    let run_row = run::Entity::find_by_id(identity.run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {}", identity.run_id)))?;
    let execution_row = execution::Entity::find_by_id(identity.execution_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent execution 不存在: {}", identity.execution_id))
        })?;
    let approval_row = approval::Entity::find_by_id(approval_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent approval 不存在: {approval_id}")))?;
    let timestamp = now();
    if approval_row.status != "pending"
        || approval_row
            .expires_at
            .is_none_or(|expires_at| expires_at > timestamp)
    {
        tx.commit().await?;
        return Ok(None);
    }
    if execution_row.run_id != run_row.id
        || approval_row.thread_id != run_row.thread_id
        || execution_row.status != "awaiting_user"
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent approval 关联 execution 状态不一致: {approval_id}"
        )));
    }
    let mut approval_active: approval::ActiveModel = approval_row.into();
    approval_active.status = Set("expired".to_string());
    approval_active.decision_note = Set(Some("Agent approval 已过期".to_string()));
    approval_active.decided_at = Set(Some(timestamp));
    let expired = approval_active.update(&tx).await?;

    let mut execution_active: execution::ActiveModel = execution_row.into();
    execution_active.status = Set("rejected".to_string());
    execution_active.error_message = Set(Some("Agent approval 已过期".to_string()));
    execution_active.updated_at = Set(timestamp);
    execution_active.completed_at = Set(Some(timestamp));
    execution_active.update(&tx).await?;

    if run_row.is_active
        && run_row.status == "waiting"
        && run_row.waiting_reason.as_deref() == Some("approval")
    {
        let mut run_active: run::ActiveModel = run_row.into();
        run_active.status = Set("queued".to_string());
        run_active.waiting_reason = Set(None);
        run_active.worker_id = Set(None);
        run_active.lease_token = Set(None);
        run_active.lease_expires_at = Set(None);
        run_active.next_attempt_at = Set(timestamp);
        run_active.updated_at = Set(timestamp);
        let run_row = run_active.update(&tx).await?;
        let _ = refresh_thread_status_on(&tx, run_row.thread_id).await?;
    } else {
        let _ = refresh_thread_status_on(&tx, run_identity.thread_id).await?;
    }
    append_event_on(
        &tx,
        AgentEventInput {
            thread_id: run_identity.thread_id,
            turn_id: Some(run_identity.turn_id),
            run_id: Some(run_identity.id),
            execution_id: Some(identity.execution_id),
            delegation_id: None,
            message_id: None,
            actor_kind: "system".into(),
            actor_id: None,
            kind: "approval_expired".into(),
            status: "expired".into(),
            public_text: "Agent 操作审批已过期".into(),
            payload_json: Some(json!({ "approval_id": expired.id.to_string() })),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(Some(expired))
}
