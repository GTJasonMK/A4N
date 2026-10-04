use super::*;
use sea_orm::ConnectionTrait;

/// Atomically appends the input message and creates the first execution run.
/// Replaying the same client id returns the original turn/run without adding
/// another active writer to the thread.
pub async fn create_turn_with_run(
    db: &impl crate::DbExecutor,
    input: AgentTurnInput,
) -> Result<(AgentTurnRecord, AgentRunRecord, AgentMessageRecord), AgentDbError> {
    let tx = db.begin().await?;
    let rows = create_turn_with_run_on(&tx, input).await?;
    tx.commit().await?;
    Ok((rows.0.into(), rows.1.into(), rows.2.into()))
}

pub(crate) async fn create_turn_with_run_on<C>(
    db: &C,
    input: AgentTurnInput,
) -> Result<(turn::Model, run::Model, message::Model), AgentDbError>
where
    C: ConnectionTrait,
{
    if input.client_message_id.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "client_message_id 不能为空".into(),
        ));
    }
    if input.message.content.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument("Agent 消息不能为空".into()));
    }
    let thread_row = thread::Entity::find_by_id(input.thread_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", input.thread_id))
        })?;

    if let Some(existing_turn) = turn::Entity::find()
        .filter(turn::Column::ThreadId.eq(input.thread_id))
        .filter(turn::Column::ClientMessageId.eq(&input.client_message_id))
        .one(db)
        .await?
    {
        let existing_run = run::Entity::find()
            .filter(run::Column::TurnId.eq(existing_turn.id))
            .order_by_desc(run::Column::Attempt)
            .one(db)
            .await?
            .ok_or_else(|| AgentDbError::NotFound("已有 Agent turn 缺少 run".into()))?;
        let existing_message = message::Entity::find_by_id(existing_turn.input_message_id)
            .one(db)
            .await?
            .ok_or_else(|| AgentDbError::NotFound("已有 Agent turn 缺少输入消息".into()))?;
        if existing_message.content_sha256 != sha256_hex(&input.message.content)
            || existing_message.role != input.message.role
        {
            return Err(AgentDbError::Conflict(format!(
                "client_message_id 已用于不同消息: {}",
                input.client_message_id
            )));
        }
        return Ok((existing_turn, existing_run, existing_message));
    }
    let active_rows = run::Entity::find()
        .filter(run::Column::ThreadId.eq(input.thread_id))
        .filter(run::Column::IsActive.eq(true))
        .lock_exclusive()
        .all(db)
        .await?;
    if active_rows.iter().any(|row| is_run_active(&row.status)) {
        return Err(AgentDbError::Conflict(
            "Agent thread 已有运行中的 turn".into(),
        ));
    }
    for row in active_rows {
        let mut inactive: run::ActiveModel = row.into();
        inactive.is_active = Set(false);
        inactive.worker_id = Set(None);
        inactive.lease_token = Set(None);
        inactive.lease_expires_at = Set(None);
        inactive.update(db).await?;
    }

    let mut message_input = input.message;
    message_input.thread_id = input.thread_id;
    message_input.client_message_id = Some(input.client_message_id.clone());
    let input_message = append_message_on(db, message_input).await?;
    let timestamp = now();
    let sequence = thread_row
        .next_turn_sequence
        .checked_add(1)
        .ok_or_else(|| AgentDbError::InvalidArgument("Agent turn sequence 溢出".into()))?;
    let turn_row = turn::ActiveModel {
        thread_id: Set(input.thread_id),
        sequence: Set(sequence),
        client_message_id: Set(input.client_message_id),
        input_message_id: Set(input_message.id),
        final_message_id: Set(None),
        trigger: Set(input.trigger),
        goal: Set(input.goal),
        status: Set("queued".to_string()),
        execution_mode: Set(input.execution_mode),
        model_override_json: Set(input.model_override_json),
        error_message: Set(None),
        result_refs_json: Set(None),
        created_at: Set(timestamp),
        updated_at: Set(timestamp),
        completed_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await?;
    let run_row = run::ActiveModel {
        thread_id: Set(input.thread_id),
        turn_id: Set(turn_row.id),
        root_run_id: Set(None),
        parent_run_id: Set(None),
        attempt: Set(1),
        status: Set("queued".to_string()),
        waiting_reason: Set(None),
        is_active: Set(true),
        next_execution_sequence: Set(0),
        budget_json: Set(input.budget_json),
        usage_json: Set(json!({
            "runtime_snapshot_required": input.runtime_snapshot_required,
        })),
        worker_id: Set(None),
        lease_token: Set(None),
        lease_expires_at: Set(None),
        next_attempt_at: Set(timestamp),
        last_error: Set(None),
        created_at: Set(timestamp),
        updated_at: Set(timestamp),
        started_at: Set(None),
        completed_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await?;
    let mut input_message_active: message::ActiveModel = input_message.into();
    input_message_active.turn_id = Set(Some(turn_row.id));
    input_message_active.run_id = Set(Some(run_row.id));
    input_message_active.updated_at = Set(timestamp);
    let input_message = input_message_active.update(db).await?;
    let mut thread_active: thread::ActiveModel = thread_row.into();
    thread_active.status = Set("running".to_string());
    let revision = thread_active
        .revision
        .take()
        .ok_or_else(|| AgentDbError::InvalidArgument("Agent thread revision 未设置".into()))?;
    thread_active.revision = Set(revision
        .checked_add(1)
        .ok_or_else(|| AgentDbError::InvalidArgument("Agent thread revision 溢出".into()))?);
    thread_active.next_turn_sequence = Set(sequence);
    thread_active.updated_at = Set(timestamp);
    thread_active.update(db).await?;
    Ok((turn_row, run_row, input_message))
}

pub async fn append_event(
    db: &impl crate::DbExecutor,
    input: AgentEventInput,
) -> Result<AgentEventRecord, AgentDbError> {
    let tx = db.begin().await?;
    let row = append_event_on(&tx, input).await?;
    tx.commit().await?;
    Ok(row.into())
}

pub(crate) async fn append_event_on<C>(
    db: &C,
    input: AgentEventInput,
) -> Result<event::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    let thread_row = thread::Entity::find_by_id(input.thread_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", input.thread_id))
        })?;
    let sequence = thread_row
        .next_event_sequence
        .checked_add(1)
        .ok_or_else(|| AgentDbError::InvalidArgument("Agent event sequence 溢出".into()))?;
    let timestamp = now();
    let row = event::ActiveModel {
        thread_id: Set(input.thread_id),
        sequence: Set(sequence),
        turn_id: Set(input.turn_id),
        run_id: Set(input.run_id),
        execution_id: Set(input.execution_id),
        delegation_id: Set(input.delegation_id),
        message_id: Set(input.message_id),
        actor_kind: Set(input.actor_kind),
        actor_id: Set(input.actor_id),
        kind: Set(input.kind),
        status: Set(input.status),
        public_text: Set(input.public_text),
        payload_json: Set(input.payload_json),
        created_at: Set(timestamp),
        ..Default::default()
    }
    .insert(db)
    .await?;
    let mut active: thread::ActiveModel = thread_row.into();
    active.next_event_sequence = Set(sequence);
    active.updated_at = Set(timestamp);
    active.update(db).await?;
    Ok(row)
}

pub(crate) async fn append_run_event_on<C>(
    db: &C,
    run_row: &run::Model,
    kind: &str,
    status: &str,
    public_text: &str,
    execution_id: Option<i64>,
    delegation_id: Option<i64>,
    message_id: Option<i64>,
    payload_json: Option<Value>,
) -> Result<event::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    append_event_on(
        db,
        AgentEventInput {
            thread_id: run_row.thread_id,
            turn_id: Some(run_row.turn_id),
            run_id: Some(run_row.id),
            execution_id,
            delegation_id,
            message_id,
            actor_kind: "agent".into(),
            actor_id: Some(run_row.id.to_string()),
            kind: kind.into(),
            status: status.into(),
            public_text: public_text.into(),
            payload_json,
        },
    )
    .await
}

/// Appends a public worker event only while the caller still owns the Run.
///
/// The Thread is locked before the Run, matching turn admission and all
/// terminal transitions that also refresh the Thread projection.
pub async fn append_event_for_run_lease(
    db: &impl crate::DbExecutor,
    run_id: i64,
    lease_token: &str,
    input: AgentEventInput,
) -> Result<AgentEventRecord, AgentDbError> {
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    if input.thread_id != identity.thread_id || input.run_id != Some(run_id) {
        return Err(AgentDbError::Conflict(format!(
            "Agent event 与 run 不一致: {run_id}"
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
    let run_row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    validate_live_run_lease(&run_row, lease_token, now())?;
    let row = append_event_on(&tx, input).await?;
    tx.commit().await?;
    Ok(row.into())
}

pub async fn events_after<C>(
    db: &C,
    thread_id: i64,
    after_sequence: i64,
    limit: u64,
) -> Result<Vec<AgentEventRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    event::Entity::find()
        .filter(event::Column::ThreadId.eq(thread_id))
        .filter(event::Column::Sequence.gt(after_sequence))
        .order_by_asc(event::Column::Sequence)
        .limit(limit)
        .all(db)
        .await
        .map(|rows| rows.into_iter().map(AgentEventRecord::from).collect())
        .map_err(AgentDbError::from)
}

pub async fn latest_events_by_thread<C>(
    db: &C,
    thread_id: i64,
    limit: u64,
) -> Result<Vec<AgentEventRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut rows = event::Entity::find()
        .filter(event::Column::ThreadId.eq(thread_id))
        .order_by_desc(event::Column::Sequence)
        .limit(limit)
        .all(db)
        .await
        .map_err(AgentDbError::from)?;
    rows.reverse();
    Ok(rows.into_iter().map(AgentEventRecord::from).collect())
}
