use super::*;
use a4n_db_models::models::agent::inbox as inbox_model;
use sea_orm::ConnectionTrait;

pub async fn create_thread<C>(
    db: &C,
    input: AgentThreadCreateInput,
) -> Result<AgentThreadRecord, AgentDbError>
where
    C: ConnectionTrait,
{
    if input.owner_user_id <= 0 {
        return Err(AgentDbError::InvalidArgument("owner_user_id 无效".into()));
    }
    if input.profile_id.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument("profile_id 不能为空".into()));
    }
    let timestamp = now();
    thread::ActiveModel {
        owner_user_id: Set(input.owner_user_id),
        thread_kind: Set(input.thread_kind),
        profile_id: Set(input.profile_id),
        title: Set(input.title),
        project_id: Set(input.project_id),
        resource_type: Set(input.resource_type),
        resource_id: Set(input.resource_id),
        canonical_binding_key: Set(input.canonical_binding_key),
        scope_json: Set(input.scope_json),
        default_model_config_id: Set(input.default_model_config_id),
        status: Set("idle".to_string()),
        is_default: Set(input.is_default),
        revision: Set(1),
        next_turn_sequence: Set(0),
        next_event_sequence: Set(0),
        context_summary: Set(None),
        created_at: Set(timestamp),
        updated_at: Set(timestamp),
        archived_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .map(AgentThreadRecord::from)
    .map_err(AgentDbError::from)
}

pub async fn thread_by_id<C>(db: &C, id: i64) -> Result<Option<AgentThreadRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    thread::Entity::find_by_id(id)
        .one(db)
        .await
        .map(|row| row.map(AgentThreadRecord::from))
        .map_err(AgentDbError::from)
}

pub async fn list_threads_by_owner<C>(
    db: &C,
    owner_user_id: i64,
    include_archived: bool,
    capability: Option<&str>,
    limit: u64,
) -> Result<Vec<AgentThreadRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut query = thread::Entity::find().filter(thread::Column::OwnerUserId.eq(owner_user_id));
    if !include_archived {
        query = query.filter(thread::Column::ArchivedAt.is_null());
    }
    if let Some(capability) = capability {
        query = query.filter(thread::Column::ProfileId.eq(capability));
    }
    query
        .order_by_desc(thread::Column::UpdatedAt)
        .order_by_desc(thread::Column::Id)
        .limit(limit)
        .all(db)
        .await
        .map(|rows| rows.into_iter().map(AgentThreadRecord::from).collect())
        .map_err(AgentDbError::from)
}

pub async fn default_thread_by_owner<C>(
    db: &C,
    owner_user_id: i64,
) -> Result<Option<AgentThreadRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    thread::Entity::find()
        .filter(thread::Column::OwnerUserId.eq(owner_user_id))
        .filter(thread::Column::IsDefault.eq(true))
        .filter(thread::Column::ArchivedAt.is_null())
        .one(db)
        .await
        .map(|row| row.map(AgentThreadRecord::from))
        .map_err(AgentDbError::from)
}

pub async fn thread_by_canonical_binding<C>(
    db: &C,
    owner_user_id: i64,
    canonical_binding_key: &str,
) -> Result<Option<AgentThreadRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    thread::Entity::find()
        .filter(thread::Column::OwnerUserId.eq(owner_user_id))
        .filter(thread::Column::CanonicalBindingKey.eq(canonical_binding_key))
        .filter(thread::Column::ArchivedAt.is_null())
        .one(db)
        .await
        .map(|row| row.map(AgentThreadRecord::from))
        .map_err(AgentDbError::from)
}

pub async fn messages_by_thread<C>(
    db: &C,
    thread_id: i64,
    before_id: Option<i64>,
    limit: u64,
) -> Result<Vec<AgentMessageRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    messages_before(db, thread_id, before_id, limit).await
}

pub async fn messages_before<C: ConnectionTrait>(
    db: &C,
    thread_id: i64,
    before: Option<i64>,
    limit: u64,
) -> Result<Vec<AgentMessageRecord>, AgentDbError> {
    let mut query = message::Entity::find()
        .filter(message::Column::ThreadId.eq(thread_id))
        .order_by_desc(message::Column::Id)
        .limit(limit);
    if let Some(before) = before {
        query = query.filter(message::Column::Id.lt(before));
    }
    let mut rows = query.all(db).await.map_err(AgentDbError::from)?;
    rows.reverse();
    Ok(rows.into_iter().map(AgentMessageRecord::from).collect())
}

pub async fn message_by_id<C>(db: &C, id: i64) -> Result<Option<AgentMessageRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    message::Entity::find_by_id(id)
        .one(db)
        .await
        .map(|row| row.map(AgentMessageRecord::from))
        .map_err(AgentDbError::from)
}

pub async fn message_by_run_and_client_id<C: ConnectionTrait>(
    db: &C,
    run_id: i64,
    client_message_id: &str,
) -> Result<Option<AgentMessageRecord>, AgentDbError> {
    message::Entity::find()
        .filter(message::Column::RunId.eq(run_id))
        .filter(message::Column::ClientMessageId.eq(client_message_id))
        .one(db)
        .await
        .map(|row| row.map(AgentMessageRecord::from))
        .map_err(AgentDbError::from)
}

pub async fn has_user_message_after<C: ConnectionTrait>(
    db: &C,
    run_id: i64,
    message_id: i64,
) -> Result<bool, AgentDbError> {
    message::Entity::find()
        .filter(message::Column::RunId.eq(run_id))
        .filter(message::Column::Role.eq("user"))
        .filter(message::Column::Id.gt(message_id))
        .one(db)
        .await
        .map(|message| message.is_some())
        .map_err(AgentDbError::from)
}

pub async fn messages_by_run<C>(
    db: &C,
    run_id: i64,
) -> Result<Vec<AgentMessageRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    message::Entity::find()
        .filter(message::Column::RunId.eq(run_id))
        .order_by_asc(message::Column::Id)
        .all(db)
        .await
        .map(|rows| rows.into_iter().map(AgentMessageRecord::from).collect())
        .map_err(AgentDbError::from)
}

/// Full unsummarized history plus the active Run's records needed for replay.
/// UI pagination must not silently discard instructions from model context.
pub async fn messages_for_context<C: ConnectionTrait>(
    db: &C,
    thread_id: i64,
    run_id: i64,
    covered_message_id: i64,
) -> Result<Vec<AgentMessageRecord>, AgentDbError> {
    message::Entity::find()
        .filter(message::Column::ThreadId.eq(thread_id))
        .filter(
            sea_orm::Condition::any()
                .add(message::Column::Id.gt(covered_message_id))
                .add(message::Column::RunId.eq(run_id)),
        )
        .order_by_asc(message::Column::Id)
        .all(db)
        .await
        .map(|rows| rows.into_iter().map(AgentMessageRecord::from).collect())
        .map_err(AgentDbError::from)
}

pub async fn turns_by_thread<C>(
    db: &C,
    thread_id: i64,
    limit: u64,
) -> Result<Vec<AgentTurnRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut rows = turn::Entity::find()
        .filter(turn::Column::ThreadId.eq(thread_id))
        .order_by_desc(turn::Column::Sequence)
        .limit(limit)
        .all(db)
        .await
        .map_err(AgentDbError::from)?;
    rows.reverse();
    Ok(rows.into_iter().map(AgentTurnRecord::from).collect())
}

pub async fn turn_by_id<C: ConnectionTrait>(
    db: &C,
    id: i64,
) -> Result<Option<AgentTurnRecord>, AgentDbError> {
    turn::Entity::find_by_id(id)
        .one(db)
        .await
        .map(|row| row.map(AgentTurnRecord::from))
        .map_err(AgentDbError::from)
}

pub async fn runs_by_thread<C>(
    db: &C,
    thread_id: i64,
    limit: u64,
) -> Result<Vec<AgentRunRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut rows = run::Entity::find()
        .filter(run::Column::ThreadId.eq(thread_id))
        .order_by_desc(run::Column::CreatedAt)
        .order_by_desc(run::Column::Id)
        .limit(limit)
        .all(db)
        .await
        .map_err(AgentDbError::from)?;
    rows.reverse();
    Ok(rows.into_iter().map(AgentRunRecord::from).collect())
}

pub async fn active_runs_by_thread<C>(
    db: &C,
    thread_id: i64,
) -> Result<Vec<AgentRunRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    Ok(run::Entity::find()
        .filter(run::Column::ThreadId.eq(thread_id))
        .order_by_desc(run::Column::UpdatedAt)
        .order_by_desc(run::Column::Id)
        .all(db)
        .await
        .map_err(AgentDbError::from)?
        .into_iter()
        .filter(|row| is_run_active(&row.status))
        .map(AgentRunRecord::from)
        .collect())
}

pub async fn run_by_id<C>(db: &C, id: i64) -> Result<Option<AgentRunRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    run::Entity::find_by_id(id)
        .one(db)
        .await
        .map(|row| row.map(AgentRunRecord::from))
        .map_err(AgentDbError::from)
}

pub async fn executions_by_runs<C>(
    db: &C,
    run_ids: Vec<i64>,
) -> Result<Vec<AgentExecutionRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    if run_ids.is_empty() {
        return Ok(Vec::new());
    }
    execution::Entity::find()
        .filter(execution::Column::RunId.is_in(run_ids))
        .order_by_asc(execution::Column::RunId)
        .order_by_asc(execution::Column::Sequence)
        .all(db)
        .await
        .map(|rows| rows.into_iter().map(AgentExecutionRecord::from).collect())
        .map_err(AgentDbError::from)
}

pub async fn execution_by_id<C: ConnectionTrait>(
    db: &C,
    id: i64,
) -> Result<Option<AgentExecutionRecord>, AgentDbError> {
    execution::Entity::find_by_id(id)
        .one(db)
        .await
        .map(|row| row.map(AgentExecutionRecord::from))
        .map_err(AgentDbError::from)
}

pub async fn execution_by_key<C: ConnectionTrait>(
    db: &C,
    run_id: i64,
    key: &str,
) -> Result<Option<AgentExecutionRecord>, AgentDbError> {
    execution::Entity::find()
        .filter(execution::Column::RunId.eq(run_id))
        .filter(execution::Column::ExecutionKey.eq(key))
        .one(db)
        .await
        .map(|row| row.map(AgentExecutionRecord::from))
        .map_err(AgentDbError::from)
}

pub async fn delegations_by_thread<C>(
    db: &C,
    thread_id: i64,
) -> Result<Vec<AgentDelegationRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    delegation::Entity::find()
        .filter(
            sea_orm::Condition::any()
                .add(delegation::Column::ParentThreadId.eq(thread_id))
                .add(delegation::Column::ChildThreadId.eq(thread_id)),
        )
        .order_by_asc(delegation::Column::CreatedAt)
        .all(db)
        .await
        .map(|rows| rows.into_iter().map(AgentDelegationRecord::from).collect())
        .map_err(AgentDbError::from)
}

pub async fn delegation_by_id<C>(
    db: &C,
    id: i64,
) -> Result<Option<AgentDelegationRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    delegation::Entity::find_by_id(id)
        .one(db)
        .await
        .map(|row| row.map(AgentDelegationRecord::from))
        .map_err(AgentDbError::from)
}

pub async fn delegation_by_parent_and_key<C>(
    db: &C,
    parent_run_id: i64,
    delegation_key: &str,
) -> Result<Option<AgentDelegationRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    delegation::Entity::find()
        .filter(delegation::Column::ParentRunId.eq(parent_run_id))
        .filter(delegation::Column::DelegationKey.eq(delegation_key))
        .one(db)
        .await
        .map(|row| row.map(AgentDelegationRecord::from))
        .map_err(AgentDbError::from)
}

pub async fn inbox_by_thread<C>(
    db: &C,
    thread_id: i64,
) -> Result<Vec<AgentInboxRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    inbox_model::Entity::find()
        .filter(inbox_model::Column::ThreadId.eq(thread_id))
        .order_by_asc(inbox_model::Column::CreatedAt)
        .all(db)
        .await
        .map(|rows| rows.into_iter().map(AgentInboxRecord::from).collect())
        .map_err(AgentDbError::from)
}

pub async fn inbox_by_id<C: ConnectionTrait>(
    db: &C,
    id: i64,
) -> Result<Option<AgentInboxRecord>, AgentDbError> {
    inbox_model::Entity::find_by_id(id)
        .one(db)
        .await
        .map(|row| row.map(AgentInboxRecord::from))
        .map_err(AgentDbError::from)
}

pub async fn latest_event_by_run_and_kind<C: ConnectionTrait>(
    db: &C,
    run_id: i64,
    kind: &str,
) -> Result<Option<AgentEventRecord>, AgentDbError> {
    event::Entity::find()
        .filter(event::Column::RunId.eq(run_id))
        .filter(event::Column::Kind.eq(kind))
        .order_by_desc(event::Column::Sequence)
        .one(db)
        .await
        .map(|row| row.map(AgentEventRecord::from))
        .map_err(AgentDbError::from)
}

pub async fn approvals_by_thread<C>(
    db: &C,
    thread_id: i64,
) -> Result<Vec<AgentApprovalRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    approval::Entity::find()
        .filter(approval::Column::ThreadId.eq(thread_id))
        .order_by_asc(approval::Column::CreatedAt)
        .all(db)
        .await
        .map(|rows| rows.into_iter().map(AgentApprovalRecord::from).collect())
        .map_err(AgentDbError::from)
}

pub async fn approval_by_id<C>(db: &C, id: i64) -> Result<Option<AgentApprovalRecord>, AgentDbError>
where
    C: ConnectionTrait,
{
    approval::Entity::find_by_id(id)
        .one(db)
        .await
        .map(|row| row.map(AgentApprovalRecord::from))
        .map_err(AgentDbError::from)
}
