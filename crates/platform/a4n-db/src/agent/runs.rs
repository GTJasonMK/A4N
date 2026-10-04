use super::*;
use a4n_db_models::models::agent::inbox as inbox_model;
use sea_orm::ConnectionTrait;

pub async fn claim_runnable_run(
    db: &impl crate::DbExecutor,
    worker_id: &str,
) -> Result<Option<AgentRunRecord>, AgentDbError> {
    if worker_id.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument("worker_id 不能为空".into()));
    }
    let tx = db.begin().await?;
    let timestamp = now();
    let claimable = sea_orm::Condition::all()
        .add(run::Column::Status.eq("queued"))
        .add(run::Column::NextAttemptAt.lte(timestamp))
        .add(
            sea_orm::Condition::any()
                .add(run::Column::LeaseExpiresAt.is_null())
                .add(run::Column::LeaseExpiresAt.lte(timestamp)),
        );
    let row = run::Entity::find()
        .filter(run::Column::IsActive.eq(true))
        .filter(claimable)
        .order_by_asc(run::Column::NextAttemptAt)
        .order_by_asc(run::Column::Id)
        .lock_with_behavior(LockType::Update, LockBehavior::SkipLocked)
        .one(&tx)
        .await?;
    let Some(row) = row else {
        tx.commit().await?;
        return Ok(None);
    };
    let token = uuid::Uuid::new_v4().to_string();
    let started_at = row.started_at.or(Some(timestamp));
    let mut active: run::ActiveModel = row.into();
    active.worker_id = Set(Some(worker_id.to_string()));
    active.lease_token = Set(Some(token));
    active.lease_expires_at = Set(Some(timestamp + Duration::seconds(RUN_LEASE_SECONDS)));
    active.status = Set("running".to_string());
    active.started_at = Set(started_at);
    active.updated_at = Set(timestamp);
    let row = active.update(&tx).await?;
    tx.commit().await?;
    Ok(Some(row.into()))
}

pub async fn renew_run_lease<C>(db: &C, run_id: i64, lease_token: &str) -> Result<(), AgentDbError>
where
    C: ConnectionTrait,
{
    let timestamp = now();
    let result = run::Entity::update_many()
        .col_expr(
            run::Column::LeaseExpiresAt,
            sea_orm::sea_query::Expr::value(timestamp + Duration::seconds(RUN_LEASE_SECONDS)),
        )
        .col_expr(
            run::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(timestamp),
        )
        .filter(run::Column::Id.eq(run_id))
        .filter(run::Column::IsActive.eq(true))
        .filter(run::Column::Status.eq("running"))
        .filter(run::Column::LeaseToken.eq(lease_token))
        .filter(run::Column::LeaseExpiresAt.gt(timestamp))
        .exec(db)
        .await?;
    if result.rows_affected != 1 {
        return Err(AgentDbError::Conflict(format!(
            "Agent run lease 已失效: {run_id}"
        )));
    }
    Ok(())
}

/// The original editor may resolve only this waiting execution. It temporarily
/// owns the same Run lease; the normal worker cannot dispatch it concurrently.
pub async fn claim_editor_execution(
    db: &impl crate::DbExecutor,
    user_id: i64,
    run_id: i64,
    execution_id: i64,
) -> Result<AgentRunRecord, AgentDbError> {
    resolve_editor_wait(db, user_id, run_id, execution_id, false)
        .await
        .map(Into::into)
}

/// Reconcile durable editor waits after disconnect or restart. Expiry and a
/// live response contend on the same locks; only one can resolve the request.
pub async fn expire_editor_requests(
    db: &impl crate::DbExecutor,
    limit: u64,
) -> Result<usize, AgentDbError> {
    let rows = run::Entity::find()
        .filter(run::Column::IsActive.eq(true))
        .filter(run::Column::Status.eq("waiting"))
        .filter(run::Column::WaitingReason.eq("external"))
        .filter(run::Column::UpdatedAt.lte(now() - Duration::seconds(EDITOR_RESPONSE_TTL_SECONDS)))
        .filter(sea_orm::sea_query::Expr::cust("jsonb_typeof((SELECT payload_json->'editor_request' FROM agent_event WHERE run_id = agent_run.id AND kind = 'run_waiting' ORDER BY sequence DESC LIMIT 1)) = 'object'"))
        .order_by_asc(run::Column::UpdatedAt).limit(limit).all(db).await?;
    let mut expired = 0;
    for row in rows {
        let request = latest_editor_wait(db, row.id).await?;
        let execution_id = request
            .payload_json
            .as_ref()
            .and_then(|value| value.pointer("/editor_request/execution_id"))
            .and_then(Value::as_str)
            .and_then(|id| id.parse::<i64>().ok())
            .ok_or_else(|| AgentDbError::InvalidArgument("编辑器等待缺少 execution_id".into()))?;
        let owner = thread::Entity::find_by_id(row.thread_id)
            .one(db)
            .await?
            .ok_or_else(|| AgentDbError::NotFound("Agent Thread 不存在".into()))?;
        match resolve_editor_wait(db, owner.owner_user_id, row.id, execution_id, true).await {
            Ok(_) => expired += 1,
            Err(AgentDbError::Conflict(_)) => {} // A response or cancellation won.
            Err(error) => return Err(error),
        }
    }
    Ok(expired)
}

pub(crate) async fn latest_editor_wait<C: ConnectionTrait>(
    db: &C,
    run_id: i64,
) -> Result<event::Model, AgentDbError> {
    event::Entity::find()
        .filter(event::Column::RunId.eq(run_id))
        .filter(event::Column::Kind.eq("run_waiting"))
        .order_by_desc(event::Column::Sequence)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::Conflict("Run 没有待响应的编辑器请求".into()))
}

pub(crate) async fn resolve_editor_wait(
    db: &impl crate::DbExecutor,
    user_id: i64,
    run_id: i64,
    execution_id: i64,
    expire: bool,
) -> Result<run::Model, AgentDbError> {
    let identity = run_by_id(db, run_id)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent Run 不存在".into()))?;
    let tx = db.begin().await?;
    let thread = thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent Thread 不存在".into()))?;
    if thread.owner_user_id != user_id {
        return Err(AgentDbError::Conflict(
            "编辑器不能响应其他用户的执行".into(),
        ));
    }
    let run = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent Run 不存在".into()))?;
    if !run.is_active
        || run.status != "waiting"
        || run.waiting_reason.as_deref() != Some("external")
    {
        return Err(AgentDbError::Conflict("编辑器请求已失效或正在处理".into()));
    }
    let wait = latest_editor_wait(&tx, run_id).await?;
    let timestamp = now();
    if expire != (timestamp >= wait.created_at + Duration::seconds(EDITOR_RESPONSE_TTL_SECONDS)) {
        return Err(AgentDbError::Conflict(
            if expire {
                "编辑器请求尚未过期"
            } else {
                "原编辑器响应已过期"
            }
            .into(),
        ));
    }
    let request = wait
        .payload_json
        .and_then(|payload| payload.get("editor_request").cloned());
    if request
        .as_ref()
        .and_then(|request| request.get("execution_id"))
        .and_then(Value::as_str)
        != Some(execution_id.to_string().as_str())
    {
        return Err(AgentDbError::Conflict("不是当前等待的编辑器执行".into()));
    }
    let execution = execution::Entity::find_by_id(execution_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent Execution 不存在".into()))?;
    if execution.run_id != run_id
        || !matches!(
            execution.status.as_str(),
            "prepared" | "awaiting_user" | "dispatching"
        )
    {
        return Err(AgentDbError::Conflict(
            "编辑器执行已结束或不属于当前任务".into(),
        ));
    }
    if expire {
        let error_message = "原编辑器离线或未及时响应，请回到发起请求的视图后重试；未执行写入";
        let mut active: execution::ActiveModel = execution.into();
        active.status = Set("failed".into());
        active.error_message = Set(Some(error_message.into()));
        active.updated_at = Set(timestamp);
        active.completed_at = Set(Some(timestamp));
        active.update(&tx).await?;
        append_run_event_on(
            &tx,
            &run,
            "editor_request_expired",
            "failed",
            "原编辑器未及时响应，工具未执行",
            Some(execution_id),
            None,
            None,
            None,
        )
        .await?;
        let failed = fail_run_on(&tx, run, Some(error_message.to_string()), timestamp).await?;
        tx.commit().await?;
        return Ok(failed);
    }
    let mut active: run::ActiveModel = run.into();
    active.status = Set("running".into());
    active.waiting_reason = Set(None);
    active.worker_id = Set(Some("editor-response".into()));
    active.lease_token = Set(Some(uuid::Uuid::new_v4().to_string()));
    active.lease_expires_at = Set(Some(timestamp + Duration::seconds(RUN_LEASE_SECONDS)));
    active.updated_at = Set(timestamp);
    let run = active.update(&tx).await?;
    tx.commit().await?;
    Ok(run)
}

pub(crate) async fn refresh_thread_status_on<C>(
    db: &C,
    thread_id: i64,
) -> Result<thread::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    let thread_row = thread::Entity::find_by_id(thread_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent thread 不存在: {thread_id}")))?;
    if thread_row.archived_at.is_some() || thread_row.status == "archived" {
        return Ok(thread_row);
    }
    let active_runs = active_runs_by_thread(db, thread_id).await?;
    if active_runs.len() > 1 {
        return Err(AgentDbError::Conflict(format!(
            "Agent thread 存在多个 active run: {thread_id}"
        )));
    }
    let status = projected_thread_status(active_runs.first())?;
    if thread_row.status == status {
        return Ok(thread_row);
    }
    let timestamp = now();
    let mut active: thread::ActiveModel = thread_row.into();
    active.status = Set(status.to_string());
    active.updated_at = Set(timestamp);
    let revision = active
        .revision
        .take()
        .ok_or_else(|| AgentDbError::InvalidArgument("Agent thread revision 未设置".into()))?;
    active.revision = Set(revision
        .checked_add(1)
        .ok_or_else(|| AgentDbError::InvalidArgument("Agent thread revision 溢出".into()))?);
    let updated = active.update(db).await.map_err(AgentDbError::from)?;
    Ok(updated)
}

pub(crate) async fn lock_threads_on<C>(
    db: &C,
    thread_ids: impl IntoIterator<Item = i64>,
) -> Result<(), AgentDbError>
where
    C: ConnectionTrait,
{
    let mut thread_ids = thread_ids.into_iter().collect::<Vec<_>>();
    thread_ids.sort_unstable();
    thread_ids.dedup();
    for thread_id in thread_ids {
        thread::Entity::find_by_id(thread_id)
            .lock_exclusive()
            .one(db)
            .await?
            .ok_or_else(|| AgentDbError::NotFound(format!("Agent thread 不存在: {thread_id}")))?;
    }
    Ok(())
}

pub(crate) async fn lock_runs_on<C>(
    db: &C,
    run_ids: impl IntoIterator<Item = i64>,
) -> Result<Vec<run::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut run_ids = run_ids.into_iter().collect::<Vec<_>>();
    run_ids.sort_unstable();
    run_ids.dedup();
    let mut rows = Vec::with_capacity(run_ids.len());
    for run_id in run_ids {
        rows.push(
            run::Entity::find_by_id(run_id)
                .lock_exclusive()
                .one(db)
                .await?
                .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?,
        );
    }
    Ok(rows)
}

pub(crate) async fn lock_turns_on<C>(
    db: &C,
    turn_ids: impl IntoIterator<Item = i64>,
) -> Result<Vec<turn::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut turn_ids = turn_ids.into_iter().collect::<Vec<_>>();
    turn_ids.sort_unstable();
    turn_ids.dedup();
    let mut rows = Vec::with_capacity(turn_ids.len());
    for turn_id in turn_ids {
        rows.push(
            turn::Entity::find_by_id(turn_id)
                .lock_exclusive()
                .one(db)
                .await?
                .ok_or_else(|| AgentDbError::NotFound(format!("Agent turn 不存在: {turn_id}")))?,
        );
    }
    Ok(rows)
}

pub(crate) async fn lock_delegations_on<C>(
    db: &C,
    delegation_ids: impl IntoIterator<Item = i64>,
) -> Result<Vec<delegation::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut delegation_ids = delegation_ids.into_iter().collect::<Vec<_>>();
    delegation_ids.sort_unstable();
    delegation_ids.dedup();
    let mut rows = Vec::with_capacity(delegation_ids.len());
    for delegation_id in delegation_ids {
        rows.push(
            delegation::Entity::find_by_id(delegation_id)
                .lock_exclusive()
                .one(db)
                .await?
                .ok_or_else(|| {
                    AgentDbError::NotFound(format!("Agent delegation 不存在: {delegation_id}"))
                })?,
        );
    }
    Ok(rows)
}

pub(crate) async fn update_run_state_on<C>(
    db: &C,
    run_id: i64,
    lease_token: Option<&str>,
    status: &str,
    waiting_reason: Option<&str>,
    error_message: Option<&str>,
) -> Result<run::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    update_run_state_on_with_options(
        db,
        run_id,
        lease_token,
        status,
        waiting_reason,
        error_message,
        None,
        None,
    )
    .await
}

async fn update_run_state_on_with_options<C>(
    db: &C,
    run_id: i64,
    lease_token: Option<&str>,
    status: &str,
    waiting_reason: Option<&str>,
    error_message: Option<&str>,
    budget_json: Option<Value>,
    expected: Option<&AgentRunRecord>,
) -> Result<run::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    validate_run_status(status)?;
    let row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    if let Some(expected) = expected
        && (row.status != expected.status
            || row.waiting_reason != expected.waiting_reason
            || row.updated_at != expected.updated_at)
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent run 状态已变化，拒绝应用过期控制请求: {run_id}"
        )));
    }
    if is_run_terminal(&row.status) && row.status != status {
        return Err(AgentDbError::Conflict(format!(
            "Agent run 已终止，不能从 {} 变为 {status}: {run_id}",
            row.status
        )));
    }
    if let Some(token) = lease_token {
        validate_live_run_lease(&row, token, now())?;
    }
    let terminal = is_run_terminal(status);
    let timestamp = now();
    let mut usage = row.usage_json.clone();
    if status != "paused"
        && let Some(usage) = usage.as_object_mut()
    {
        usage.remove("paused_by_run_id");
    }
    let mut active: run::ActiveModel = row.into();
    active.usage_json = Set(usage);
    if let Some(budget_json) = budget_json {
        active.budget_json = Set(budget_json);
    }
    active.status = Set(status.to_string());
    active.waiting_reason = Set(waiting_reason.map(str::to_string));
    active.last_error = Set(error_message.map(str::to_string));
    active.is_active = Set(!terminal);
    active.updated_at = Set(timestamp);
    if terminal {
        active.completed_at = Set(Some(timestamp));
        active.worker_id = Set(None);
        active.lease_token = Set(None);
        active.lease_expires_at = Set(None);
    } else {
        active.completed_at = Set(None);
        if matches!(status, "queued" | "waiting" | "pausing" | "paused") {
            active.worker_id = Set(None);
            active.lease_token = Set(None);
            active.lease_expires_at = Set(None);
        }
        if status == "queued" {
            active.next_attempt_at = Set(timestamp);
        }
    }
    let updated = active.update(db).await.map_err(AgentDbError::from)?;
    let lifecycle = match status {
        "queued" => Some(("run_queued", "queued", "Agent run 已排队")),
        "waiting" => Some(("run_waiting", "waiting", "Agent run 正在等待")),
        "completed" => Some(("run_completed", "completed", "Agent run 已完成")),
        "failed" => Some(("run_failed", "failed", "Agent run 执行失败")),
        "cancelled" => Some(("run_cancelled", "cancelled", "Agent run 已取消")),
        "recovery_required" => Some((
            "run_recovery_required",
            "recovery_required",
            "Agent run 需要显式恢复或对账",
        )),
        _ => None,
    };
    if let Some((kind, event_status, public_text)) = lifecycle {
        append_run_event_on(
            db,
            &updated,
            kind,
            event_status,
            public_text,
            None,
            None,
            None,
            (status == "waiting")
                .then(|| serde_json::json!({"waiting_reason": updated.waiting_reason.clone()})),
        )
        .await?;
    }
    Ok(updated)
}

pub async fn update_run_state(
    db: &impl crate::DbExecutor,
    run_id: i64,
    lease_token: Option<&str>,
    status: &str,
    waiting_reason: Option<&str>,
    error_message: Option<&str>,
) -> Result<AgentRunRecord, AgentDbError> {
    update_run_state_inner(
        db,
        run_id,
        lease_token,
        status,
        waiting_reason,
        error_message,
        None,
        None,
    )
    .await
    .map(Into::into)
}

pub async fn update_run_state_with_budget(
    db: &impl crate::DbExecutor,
    run_id: i64,
    lease_token: Option<&str>,
    status: &str,
    waiting_reason: Option<&str>,
    error_message: Option<&str>,
    budget_json: Value,
) -> Result<AgentRunRecord, AgentDbError> {
    update_run_state_inner(
        db,
        run_id,
        lease_token,
        status,
        waiting_reason,
        error_message,
        Some(budget_json),
        None,
    )
    .await
    .map(Into::into)
}

pub async fn update_run_state_if_unchanged(
    db: &impl crate::DbExecutor,
    expected: &AgentRunRecord,
    lease_token: Option<&str>,
    status: &str,
    waiting_reason: Option<&str>,
    error_message: Option<&str>,
    budget_json: Option<Value>,
) -> Result<AgentRunRecord, AgentDbError> {
    update_run_state_inner(
        db,
        expected.id,
        lease_token,
        status,
        waiting_reason,
        error_message,
        budget_json,
        Some(expected),
    )
    .await
    .map(Into::into)
}

async fn update_run_state_inner(
    db: &impl crate::DbExecutor,
    run_id: i64,
    lease_token: Option<&str>,
    status: &str,
    waiting_reason: Option<&str>,
    error_message: Option<&str>,
    budget_json: Option<Value>,
    expected: Option<&AgentRunRecord>,
) -> Result<run::Model, AgentDbError> {
    validate_run_status(status)?;
    let current = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let tx = db.begin().await?;
    thread::Entity::find_by_id(current.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", current.thread_id))
        })?;
    let run_row = update_run_state_on_with_options(
        &tx,
        run_id,
        lease_token,
        status,
        waiting_reason,
        error_message,
        budget_json,
        expected,
    )
    .await?;
    let _ = refresh_thread_status_on(&tx, run_row.thread_id).await?;
    tx.commit().await?;
    Ok(run_row)
}

/// Moves a leased worker run to `waiting` and releases the lease in one
/// transaction.  Waiting runs must not retain a worker lease: approvals and
/// child results should be able to wake them immediately.
pub async fn wait_run_for_lease(
    db: &impl crate::DbExecutor,
    run_id: i64,
    lease_token: &str,
    waiting_reason: &str,
) -> Result<AgentRunRecord, AgentDbError> {
    wait_run_for_lease_with_request(db, run_id, lease_token, waiting_reason, None).await
}

/// Publishes the external request in the same transaction that releases the
/// Run, so a fast UI response cannot arrive before the wait is durable.
pub async fn wait_run_for_lease_with_request(
    db: &impl crate::DbExecutor,
    run_id: i64,
    lease_token: &str,
    waiting_reason: &str,
    editor_request: Option<Value>,
) -> Result<AgentRunRecord, AgentDbError> {
    if waiting_reason.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "Agent waiting_reason 不能为空".into(),
        ));
    }
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let tx = db.begin().await?;
    if waiting_reason == "budget" {
        budget::lock_budget_tree_on(&tx, run_id).await?;
    }
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    let current = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let timestamp = now();
    validate_live_run_lease(&current, lease_token, timestamp)?;
    let ready = match waiting_reason {
        "budget" => budget::budget_wait_can_resume(&tx, &current).await?,
        "child" => delegation::Entity::find()
            .filter(delegation::Column::ParentRunId.eq(run_id))
            .filter(delegation::Column::Status.is_in(["pending", "running", "revision_requested"]))
            .one(&tx)
            .await?
            .is_none(),
        "approval" => approval::Entity::find()
            .filter(approval::Column::RunId.eq(run_id))
            .filter(approval::Column::Status.eq("pending"))
            .one(&tx)
            .await?
            .is_none(),
        _ => false,
    };
    if ready {
        let row = update_run_state_on(&tx, run_id, Some(lease_token), "queued", None, None).await?;
        refresh_thread_status_on(&tx, row.thread_id).await?;
        tx.commit().await?;
        return Ok(row.into());
    }
    let mut active: run::ActiveModel = current.into();
    active.status = Set("waiting".to_string());
    active.waiting_reason = Set(Some(waiting_reason.to_string()));
    active.worker_id = Set(None);
    active.lease_token = Set(None);
    active.lease_expires_at = Set(None);
    active.updated_at = Set(timestamp);
    let row = active.update(&tx).await?;
    append_run_event_on(
        &tx,
        &row,
        "run_waiting",
        "waiting",
        "Agent run 正在等待",
        None,
        None,
        None,
        Some(json!({ "waiting_reason": waiting_reason, "editor_request": editor_request })),
    )
    .await?;
    let _ = refresh_thread_status_on(&tx, row.thread_id).await?;
    tx.commit().await?;
    Ok(row.into())
}

/// Requeues an active waiting run and releases any stale lease.  This is used
/// by approval/child-result delivery and intentionally has no worker token:
/// the durable inbox is the authorization boundary for the wake-up.
/// Requeues a waiting Run only when its current wait reason matches the
/// supplied reason. User/external input may release a Run waiting for input,
/// but must not bypass an approval, child result, budget, or reconciliation
/// gate. A concurrent resolver that already changed the state is an ordinary
/// no-op rather than a failed message delivery.
pub async fn wake_waiting_run_for_reason(
    db: &impl crate::DbExecutor,
    run_id: i64,
    waiting_reason: &str,
) -> Result<Option<AgentRunRecord>, AgentDbError> {
    if waiting_reason.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "Agent waiting_reason 不能为空".into(),
        ));
    }
    let identity = run_by_id(db, run_id)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent Run 不存在".into()))?;
    let tx = db.begin().await?;
    lock_threads_on(&tx, [identity.thread_id]).await?;
    let row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    if !row.is_active
        || row.status != "waiting"
        || row.waiting_reason.as_deref() != Some(waiting_reason)
    {
        tx.commit().await?;
        return Ok(None);
    }
    let timestamp = now();
    let mut active: run::ActiveModel = row.into();
    active.status = Set("queued".to_string());
    active.waiting_reason = Set(None);
    active.worker_id = Set(None);
    active.lease_token = Set(None);
    active.lease_expires_at = Set(None);
    active.next_attempt_at = Set(timestamp);
    active.updated_at = Set(timestamp);
    let row = active.update(&tx).await?;
    append_run_event_on(
        &tx,
        &row,
        "run_woken",
        "queued",
        "Agent run 已被外部输入唤醒",
        None,
        None,
        None,
        Some(json!({ "waiting_reason": waiting_reason })),
    )
    .await?;
    let _ = refresh_thread_status_on(&tx, row.thread_id).await?;
    tx.commit().await?;
    Ok(Some(row.into()))
}

/// Fenced failure transition for a worker.  Keeping Turn and Run terminal
/// updates in one transaction avoids an observable half-failed turn.
pub(crate) async fn fail_run_on<C: ConnectionTrait>(
    db: &C,
    row: run::Model,
    error_message: Option<String>,
    timestamp: NaiveDateTime,
) -> Result<run::Model, AgentDbError> {
    let turn_row = turn::Entity::find_by_id(row.turn_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent turn 不存在: {}", row.turn_id)))?;
    let mut turn_active: turn::ActiveModel = turn_row.into();
    turn_active.status = Set("failed".to_string());
    turn_active.error_message = Set(error_message.clone());
    turn_active.updated_at = Set(timestamp);
    turn_active.completed_at = Set(Some(timestamp));
    turn_active.update(db).await?;

    let mut active: run::ActiveModel = row.into();
    active.status = Set("failed".to_string());
    active.waiting_reason = Set(None);
    active.is_active = Set(false);
    active.last_error = Set(error_message);
    active.updated_at = Set(timestamp);
    active.completed_at = Set(Some(timestamp));
    active.worker_id = Set(None);
    active.lease_token = Set(None);
    active.lease_expires_at = Set(None);
    let row = active.update(db).await?;
    terminate_executions_on(
        db,
        [row.id],
        ExecutionTermination::Failed,
        row.last_error.as_deref(),
        timestamp,
    )
    .await?;
    interrupt_streaming_messages_on(db, [row.id], row.last_error.as_deref()).await?;
    append_run_event_on(
        db,
        &row,
        "run_failed",
        "failed",
        "Agent run 执行失败",
        None,
        None,
        None,
        row.last_error
            .clone()
            .map(|error| json!({ "error": error })),
    )
    .await?;
    let _ = refresh_thread_status_on(db, row.thread_id).await?;
    Ok(row)
}

pub async fn fail_run_for_lease(
    db: &impl crate::DbExecutor,
    run_id: i64,
    lease_token: &str,
    error_message: Option<String>,
) -> Result<AgentRunRecord, AgentDbError> {
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let tx = db.begin().await?;
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    let row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let timestamp = now();
    validate_live_run_lease(&row, lease_token, timestamp)?;
    let row = fail_run_on(&tx, row, error_message, timestamp).await?;
    tx.commit().await?;
    Ok(row.into())
}

/// Fenced transition used when a worker cannot safely resume (for example the
/// process-local runtime snapshot is gone).  The state is terminal and must be
/// reconciled explicitly instead of silently retrying an unknown side effect.
pub async fn recovery_required_for_lease(
    db: &impl crate::DbExecutor,
    run_id: i64,
    lease_token: &str,
    reason: Option<String>,
) -> Result<AgentRunRecord, AgentDbError> {
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let tx = db.begin().await?;
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    let row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let timestamp = now();
    validate_live_run_lease(&row, lease_token, timestamp)?;
    let turn_row = turn::Entity::find_by_id(row.turn_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent turn 不存在: {}", row.turn_id)))?;
    let mut turn_active: turn::ActiveModel = turn_row.into();
    turn_active.status = Set("failed".to_string());
    turn_active.error_message = Set(reason.clone());
    turn_active.updated_at = Set(timestamp);
    turn_active.completed_at = Set(Some(timestamp));
    turn_active.update(&tx).await?;
    let mut active: run::ActiveModel = row.into();
    active.status = Set("recovery_required".to_string());
    active.waiting_reason = Set(Some("reconcile".to_string()));
    active.is_active = Set(false);
    active.last_error = Set(reason);
    active.updated_at = Set(timestamp);
    active.completed_at = Set(Some(timestamp));
    active.worker_id = Set(None);
    active.lease_token = Set(None);
    active.lease_expires_at = Set(None);
    let row = active.update(&tx).await?;
    terminate_executions_on(
        &tx,
        [row.id],
        ExecutionTermination::RecoveryRequired,
        row.last_error.as_deref(),
        timestamp,
    )
    .await?;
    interrupt_streaming_messages_on(&tx, [row.id], row.last_error.as_deref()).await?;
    append_run_event_on(
        &tx,
        &row,
        "run_recovery_required",
        "recovery_required",
        "Agent run 需要显式恢复或对账",
        None,
        None,
        None,
        row.last_error
            .clone()
            .map(|error| json!({ "error": error })),
    )
    .await?;
    let _ = refresh_thread_status_on(&tx, row.thread_id).await?;
    tx.commit().await?;
    Ok(row.into())
}

pub(crate) async fn complete_turn<C>(
    db: &C,
    turn_id: i64,
    status: &str,
    final_message_id: Option<i64>,
    result_refs_json: Option<Value>,
    error_message: Option<String>,
) -> Result<turn::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    validate_turn_status(status)?;
    let row = turn::Entity::find_by_id(turn_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent turn 不存在: {turn_id}")))?;
    if is_turn_terminal(&row.status) && row.status != status {
        return Err(AgentDbError::Conflict(format!(
            "Agent turn 已终止，不能从 {} 变为 {status}: {turn_id}",
            row.status
        )));
    }
    let timestamp = now();
    let mut active: turn::ActiveModel = row.into();
    active.status = Set(status.to_string());
    active.final_message_id = Set(final_message_id);
    active.result_refs_json = Set(result_refs_json);
    active.error_message = Set(error_message);
    active.updated_at = Set(timestamp);
    if is_turn_terminal(status) {
        active.completed_at = Set(Some(timestamp));
    } else {
        active.completed_at = Set(None);
    }
    active.update(db).await.map_err(AgentDbError::from)
}

/// Marks a Turn active while fencing the transition to its Run lease.  The
/// check and Turn update share one transaction, so a recovered/stale worker
/// cannot resurrect a queued Turn after its Run lease has changed hands.
pub async fn mark_turn_active_for_run_lease(
    db: &impl crate::DbExecutor,
    run_id: i64,
    lease_token: &str,
) -> Result<AgentTurnRecord, AgentDbError> {
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let tx = db.begin().await?;
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    let run_row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    validate_live_run_lease(&run_row, lease_token, now())?;
    let turn_row = turn::Entity::find_by_id(run_row.turn_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent turn 不存在: {}", run_row.turn_id)))?;
    if turn_row.thread_id != run_row.thread_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent turn 与 run 不一致: {run_id}"
        )));
    }
    if is_turn_terminal(&turn_row.status) {
        return Err(AgentDbError::Conflict(format!(
            "Agent turn 已终止，不能激活: {}",
            turn_row.id
        )));
    }
    let timestamp = now();
    let mut active: turn::ActiveModel = turn_row.into();
    active.status = Set("active".to_string());
    active.updated_at = Set(timestamp);
    active.completed_at = Set(None);
    let row = active.update(&tx).await?;
    tx.commit().await?;
    Ok(row.into())
}

/// Atomically appends an assistant final message and closes its Turn/Run.
///
/// The lease is checked while the Run row is locked, so a stale worker cannot
/// leave a completed message behind while the durable Run remains active (or
/// vice versa).
pub async fn finish_run_with_message(
    db: &impl crate::DbExecutor,
    run_id: i64,
    lease_token: &str,
    message: AgentMessageInput,
    result_refs_json: Option<Value>,
    observed_input_message_id: i64,
) -> Result<Option<(AgentRunRecord, AgentMessageRecord)>, AgentDbError> {
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let tx = db.begin().await?;
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    let run_row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let timestamp = now();
    validate_live_run_lease(&run_row, lease_token, timestamp)?;
    if message.thread_id != run_row.thread_id
        || message.turn_id != Some(run_row.turn_id)
        || message.run_id != Some(run_id)
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent final message 与 run 不一致: {run_id}"
        )));
    }
    if requeue_for_new_input_on(&tx, &run_row, observed_input_message_id).await? {
        tx.commit().await?;
        return Ok(None);
    }
    let blockers = completion_blockers(&tx, run_id).await?;
    if !blockers.is_empty() {
        return Err(AgentDbError::CompletionBlocked(blockers));
    }
    turn::Entity::find_by_id(run_row.turn_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent turn 不存在: {}", run_row.turn_id)))?;
    let final_message = if let Some(streaming) = message::Entity::find()
        .filter(message::Column::RunId.eq(run_row.id))
        .filter(message::Column::Role.eq("assistant"))
        .filter(message::Column::Status.eq("streaming"))
        .order_by_desc(message::Column::Id)
        .lock_exclusive()
        .one(&tx)
        .await?
    {
        let mut active: message::ActiveModel = streaming.into();
        active.content_sha256 = Set(sha256_hex(&message.content));
        active.content = Set(message.content);
        active.status = Set("complete".into());
        active.payload_json = Set(message.payload_json);
        active.updated_at = Set(timestamp);
        active.update(&tx).await?
    } else {
        append_message_on(&tx, message).await?
    };
    append_message_event_on(&tx, &final_message, "message_completed", "complete").await?;
    let _turn = complete_turn(
        &tx,
        run_row.turn_id,
        "completed",
        Some(final_message.id),
        result_refs_json,
        None,
    )
    .await?;
    let mut active: run::ActiveModel = run_row.into();
    active.status = Set("completed".to_string());
    active.waiting_reason = Set(None);
    active.is_active = Set(false);
    active.updated_at = Set(timestamp);
    active.completed_at = Set(Some(timestamp));
    active.worker_id = Set(None);
    active.lease_token = Set(None);
    active.lease_expires_at = Set(None);
    let completed_run = active.update(&tx).await?;
    append_run_event_on(
        &tx,
        &completed_run,
        "run_completed",
        "completed",
        "Agent run 已完成",
        None,
        None,
        Some(final_message.id),
        None,
    )
    .await?;
    let _ = refresh_thread_status_on(&tx, completed_run.thread_id).await?;
    tx.commit().await?;
    Ok(Some((completed_run.into(), final_message.into())))
}

// The sender and finalizer both hold the Thread lock. An accepted follow-up
// either belongs to this still-active Run or starts a new Turn after it closes.
pub(crate) async fn requeue_for_new_input_on<C: ConnectionTrait>(
    db: &C,
    row: &run::Model,
    observed_input_message_id: i64,
) -> Result<bool, AgentDbError> {
    if message::Entity::find()
        .filter(message::Column::RunId.eq(row.id))
        .filter(message::Column::Role.eq("user"))
        .filter(message::Column::Id.gt(observed_input_message_id))
        .one(db)
        .await?
        .is_none()
    {
        return Ok(false);
    }
    update_run_state_on(db, row.id, row.lease_token.as_deref(), "queued", None, None).await?;
    interrupt_streaming_messages_on(db, [row.id], Some("已有新的用户输入，继续处理后再交付"))
        .await?;
    refresh_thread_status_on(db, row.thread_id).await?;
    Ok(true)
}

pub(crate) async fn consume_observed_user_inbox_on<C: ConnectionTrait>(
    db: &C,
    owner: &run::Model,
    message_id: i64,
) -> Result<(), AgentDbError> {
    let rows = inbox_model::Entity::find()
        .filter(inbox_model::Column::RunId.eq(owner.id))
        .filter(inbox_model::Column::SourceKind.eq("user"))
        .filter(inbox_model::Column::Status.is_in(["pending", "claimed"]))
        .lock_exclusive()
        .all(db)
        .await?;
    for row in rows {
        let id = row
            .payload_json
            .as_ref()
            .and_then(|v| v.get("message_id"))
            .and_then(Value::as_str)
            .and_then(|v| v.parse::<i64>().ok())
            .ok_or_else(|| AgentDbError::InvalidArgument("用户 inbox 缺少 message_id".into()))?;
        if id <= message_id {
            let mut active: inbox_model::ActiveModel = row.into();
            active.status = Set("consumed".into());
            active.claim_token = Set(None);
            active.claim_expires_at = Set(None);
            active.consumed_at = Set(Some(now()));
            active.update(db).await?;
        }
    }
    Ok(())
}

pub async fn update_run_usage(
    db: &impl crate::DbExecutor,
    run_id: i64,
    lease_token: &str,
    usage_json: Value,
) -> Result<AgentRunRecord, AgentDbError> {
    let tx = db.begin().await?;
    let row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let timestamp = now();
    validate_live_run_lease(&row, lease_token, timestamp)?;
    // The process-local snapshot requirement is an admission invariant. A
    // Worker usage writes must not be able to erase it by sending a stale or
    // hand-built usage object, otherwise a later resume could run without the
    // credentials/configuration it was admitted with.
    let mut usage_json = usage_json;
    let snapshot_required = match row.usage_json.get("runtime_snapshot_required") {
        Some(Value::Bool(required)) => *required,
        Some(_) => {
            return Err(AgentDbError::InvalidArgument(format!(
                "Agent run {run_id} runtime_snapshot_required 必须是 boolean"
            )));
        }
        None => {
            return Err(AgentDbError::InvalidArgument(format!(
                "Agent run {run_id} 缺少 runtime_snapshot_required 标记"
            )));
        }
    };
    let usage_object = usage_json
        .as_object_mut()
        .ok_or_else(|| AgentDbError::InvalidArgument("Agent run usage_json 必须是对象".into()))?;
    for field in [
        "steps_used",
        "model_calls_used",
        "tool_calls_used",
        "tokens_used",
        "tokens_reserved",
        "usage_complete",
        "budget_limiting_run_id",
    ] {
        usage_object.remove(field);
    }
    usage_object.insert(
        "runtime_snapshot_required".into(),
        Value::Bool(snapshot_required),
    );
    // Control-plane wait/lineage markers cannot be erased by stale Worker
    // progress. They are not counters and are cleared by their lifecycle owner.
    for field in ["budget_wait", "paused_by_run_id"] {
        usage_object.remove(field);
        if let Some(value) = row.usage_json.get(field) {
            usage_object.insert(field.into(), value.clone());
        }
    }
    let mut active: run::ActiveModel = row.into();
    active.usage_json = Set(usage_json);
    active.updated_at = Set(timestamp);
    let row = active.update(&tx).await?;
    tx.commit().await?;
    Ok(row.into())
}
