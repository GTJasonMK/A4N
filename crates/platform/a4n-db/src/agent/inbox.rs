use super::*;
use a4n_db_models::models::agent::inbox as inbox_model;
use sea_orm::ConnectionTrait;

pub async fn enqueue_inbox<C>(
    db: &C,
    input: AgentInboxInput,
) -> Result<AgentInboxRecord, AgentDbError>
where
    C: ConnectionTrait,
{
    if input.dedupe_key.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "inbox dedupe_key 不能为空".into(),
        ));
    }
    let timestamp = now();
    let digest = sha256_hex(&input.content);
    let row = inbox_model::ActiveModel {
        thread_id: Set(input.thread_id),
        turn_id: Set(input.turn_id),
        run_id: Set(input.run_id),
        source_kind: Set(input.source_kind),
        source_thread_id: Set(input.source_thread_id),
        inbox_kind: Set(input.inbox_kind),
        delivery_mode: Set(input.delivery_mode),
        priority: Set(input.priority),
        dedupe_key: Set(input.dedupe_key.clone()),
        content_sha256: Set(digest.clone()),
        content: Set(input.content),
        payload_json: Set(input.payload_json),
        status: Set("pending".to_string()),
        available_at: Set(input.available_at.unwrap_or(timestamp)),
        claim_token: Set(None),
        claim_expires_at: Set(None),
        created_at: Set(timestamp),
        claimed_at: Set(None),
        consumed_at: Set(None),
        ..Default::default()
    };
    inbox_model::Entity::insert(row)
        .on_conflict(
            OnConflict::columns([
                inbox_model::Column::ThreadId,
                inbox_model::Column::DedupeKey,
            ])
            .do_nothing()
            .to_owned(),
        )
        .do_nothing()
        .exec_without_returning(db)
        .await?;
    let existing = inbox_model::Entity::find()
        .filter(inbox_model::Column::ThreadId.eq(input.thread_id))
        .filter(inbox_model::Column::DedupeKey.eq(&input.dedupe_key))
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "inbox 插入或幂等冲突后无法重读: thread={}, dedupe_key={}",
                input.thread_id, input.dedupe_key
            ))
        })?;
    if existing.content_sha256 != digest {
        return Err(AgentDbError::Conflict(format!(
            "inbox dedupe_key 已用于不同内容: {}",
            input.dedupe_key
        )));
    }
    Ok(existing.into())
}

pub async fn claim_inbox(
    db: &impl crate::DbExecutor,
    thread_id: Option<i64>,
) -> Result<Option<AgentInboxRecord>, AgentDbError> {
    let tx = db.begin().await?;
    let timestamp = now();
    let claimable = sea_orm::Condition::any()
        .add(
            sea_orm::Condition::all()
                .add(inbox_model::Column::Status.eq("pending"))
                .add(inbox_model::Column::AvailableAt.lte(timestamp)),
        )
        .add(
            sea_orm::Condition::all()
                .add(inbox_model::Column::Status.eq("claimed"))
                .add(inbox_model::Column::ClaimExpiresAt.lte(timestamp)),
        );
    let mut query = inbox_model::Entity::find()
        .filter(claimable)
        .order_by_desc(inbox_model::Column::Priority)
        .order_by_asc(inbox_model::Column::Id)
        .lock_with_behavior(LockType::Update, LockBehavior::SkipLocked);
    if let Some(thread_id) = thread_id {
        query = query.filter(inbox_model::Column::ThreadId.eq(thread_id));
    }
    let Some(row) = query.one(&tx).await? else {
        tx.commit().await?;
        return Ok(None);
    };
    let token = uuid::Uuid::new_v4().to_string();
    let mut active: inbox_model::ActiveModel = row.into();
    active.status = Set("claimed".to_string());
    active.claim_token = Set(Some(token));
    active.claim_expires_at = Set(Some(timestamp + Duration::seconds(INBOX_LEASE_SECONDS)));
    active.claimed_at = Set(Some(timestamp));
    let row = active.update(&tx).await?;
    tx.commit().await?;
    Ok(Some(row.into()))
}

pub(crate) fn validate_inbox_claim(
    row: &inbox_model::Model,
    claim_token: &str,
    timestamp: NaiveDateTime,
) -> Result<(), AgentDbError> {
    if claim_token.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "inbox claim_token 不能为空".into(),
        ));
    }
    if row.status != "claimed"
        || row.claim_token.as_deref() != Some(claim_token)
        || row
            .claim_expires_at
            .is_none_or(|expires_at| expires_at <= timestamp)
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent inbox claim 已失效: {}",
            row.id
        )));
    }
    Ok(())
}

pub async fn consume_inbox(
    db: &impl crate::DbExecutor,
    inbox_id: i64,
    claim_token: &str,
) -> Result<AgentInboxRecord, AgentDbError> {
    let tx = db.begin().await?;
    let row = inbox_model::Entity::find_by_id(inbox_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent inbox 不存在: {inbox_id}")))?;
    let timestamp = now();
    validate_inbox_claim(&row, claim_token, timestamp)?;
    let mut active: inbox_model::ActiveModel = row.into();
    active.status = Set("consumed".to_string());
    active.claim_token = Set(None);
    active.claim_expires_at = Set(None);
    active.consumed_at = Set(Some(timestamp));
    let row = active.update(&tx).await?;
    tx.commit().await?;
    Ok(row.into())
}

/// Atomically turns a claimed inbox item into a durable Agent message and
/// consumes the item.  The message is written in the same transaction as the
/// claim transition, so a worker crash cannot lose an observation between
/// consuming the inbox and rebuilding model history.
pub async fn consume_inbox_with_message(
    db: &impl crate::DbExecutor,
    inbox_id: i64,
    claim_token: &str,
    run_lease_token: &str,
    message: AgentMessageInput,
) -> Result<(AgentInboxRecord, AgentMessageRecord), AgentDbError> {
    let identity = inbox_model::Entity::find_by_id(inbox_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent inbox 不存在: {inbox_id}")))?;
    let tx = db.begin().await?;

    // Keep the shared lock order (Thread → Run → Inbox).  Cancellation and
    // child completion use the same order before touching these records.
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    if let Some(run_id) = identity.run_id {
        let run_row = run::Entity::find_by_id(run_id)
            .lock_exclusive()
            .one(&tx)
            .await?
            .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
        // Consuming an inbox item mutates the durable conversation history.
        // It therefore has to be fenced by the same live Run lease as every
        // other worker write; an expired worker must roll back both message
        // insertion and inbox consumption.
        validate_live_run_lease(&run_row, run_lease_token, now())?;
    }
    let row = inbox_model::Entity::find_by_id(inbox_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent inbox 不存在: {inbox_id}")))?;
    validate_inbox_claim(&row, claim_token, now())?;
    if message.thread_id != row.thread_id
        || message.turn_id != row.turn_id
        || message.run_id != row.run_id
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent inbox durable message 与 inbox 不一致: {inbox_id}"
        )));
    }
    let durable_message = append_message_on(&tx, message).await?;
    let timestamp = now();
    let mut active: inbox_model::ActiveModel = row.into();
    active.status = Set("consumed".to_string());
    active.claim_token = Set(None);
    active.claim_expires_at = Set(None);
    active.consumed_at = Set(Some(timestamp));
    let consumed = active.update(&tx).await?;
    tx.commit().await?;
    Ok((consumed.into(), durable_message.into()))
}

pub async fn supersede_inbox(
    db: &impl crate::DbExecutor,
    inbox_id: i64,
    claim_token: Option<&str>,
) -> Result<AgentInboxRecord, AgentDbError> {
    let tx = db.begin().await?;
    let row = inbox_model::Entity::find_by_id(inbox_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent inbox 不存在: {inbox_id}")))?;
    let timestamp = now();
    match row.status.as_str() {
        "pending" => {}
        "claimed" => validate_inbox_claim(
            &row,
            claim_token.ok_or_else(|| {
                AgentDbError::Conflict(format!("Agent inbox claim token 缺失: {inbox_id}"))
            })?,
            timestamp,
        )?,
        "superseded" => {
            tx.commit().await?;
            return Ok(row.into());
        }
        status => {
            return Err(AgentDbError::Conflict(format!(
                "Agent inbox 无法从 {status} 变为 superseded: {inbox_id}"
            )));
        }
    }
    let mut active: inbox_model::ActiveModel = row.into();
    active.status = Set("superseded".to_string());
    active.claim_token = Set(None);
    active.claim_expires_at = Set(None);
    active.consumed_at = Set(Some(timestamp));
    let row = active.update(&tx).await?;
    tx.commit().await?;
    Ok(row.into())
}

pub async fn release_inbox_claim(
    db: &impl crate::DbExecutor,
    inbox_id: i64,
    claim_token: &str,
    available_at: Option<NaiveDateTime>,
) -> Result<AgentInboxRecord, AgentDbError> {
    let tx = db.begin().await?;
    let row = inbox_model::Entity::find_by_id(inbox_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent inbox 不存在: {inbox_id}")))?;
    let timestamp = now();
    validate_inbox_claim(&row, claim_token, timestamp)?;
    let mut active: inbox_model::ActiveModel = row.into();
    active.status = Set("pending".to_string());
    active.available_at = Set(available_at.unwrap_or(timestamp));
    active.claim_token = Set(None);
    active.claim_expires_at = Set(None);
    active.claimed_at = Set(None);
    let row = active.update(&tx).await?;
    tx.commit().await?;
    Ok(row.into())
}
