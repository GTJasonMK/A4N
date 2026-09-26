use super::*;
use a4n_db_models::models::agent::inbox as inbox_model;

pub async fn create_thread<C>(
    db: &C,
    input: AgentThreadCreateInput,
) -> Result<thread::Model, AgentDbError>
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
    .map_err(AgentDbError::Database)
}

pub async fn thread_by_id<C>(db: &C, id: i64) -> Result<Option<thread::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    thread::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn list_threads_by_owner<C>(
    db: &C,
    owner_user_id: i64,
) -> Result<Vec<thread::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    thread::Entity::find()
        .filter(thread::Column::OwnerUserId.eq(owner_user_id))
        .order_by_desc(thread::Column::UpdatedAt)
        .order_by_desc(thread::Column::Id)
        .all(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn default_thread_by_owner<C>(
    db: &C,
    owner_user_id: i64,
) -> Result<Option<thread::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    thread::Entity::find()
        .filter(thread::Column::OwnerUserId.eq(owner_user_id))
        .filter(thread::Column::IsDefault.eq(true))
        .filter(thread::Column::ArchivedAt.is_null())
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn thread_by_canonical_binding<C>(
    db: &C,
    owner_user_id: i64,
    canonical_binding_key: &str,
) -> Result<Option<thread::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    thread::Entity::find()
        .filter(thread::Column::OwnerUserId.eq(owner_user_id))
        .filter(thread::Column::CanonicalBindingKey.eq(canonical_binding_key))
        .filter(thread::Column::ArchivedAt.is_null())
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn messages_by_thread<C>(
    db: &C,
    thread_id: i64,
    before_id: Option<i64>,
    limit: u64,
) -> Result<Vec<message::Model>, AgentDbError>
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
) -> Result<Vec<message::Model>, AgentDbError> {
    let mut query = message::Entity::find()
        .filter(message::Column::ThreadId.eq(thread_id))
        .order_by_desc(message::Column::Id)
        .limit(limit);
    if let Some(before) = before {
        query = query.filter(message::Column::Id.lt(before));
    }
    let mut rows = query.all(db).await.map_err(AgentDbError::Database)?;
    rows.reverse();
    Ok(rows)
}

pub async fn message_by_id<C>(db: &C, id: i64) -> Result<Option<message::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    message::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn message_by_client_message_id<C>(
    db: &C,
    run_id: i64,
    client_message_id: &str,
) -> Result<Option<message::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    message::Entity::find()
        .filter(message::Column::RunId.eq(run_id))
        .filter(message::Column::ClientMessageId.eq(client_message_id))
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn user_message_after<C>(
    db: &C,
    run_id: i64,
    message_id: i64,
) -> Result<Option<message::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    message::Entity::find()
        .filter(message::Column::RunId.eq(run_id))
        .filter(message::Column::Role.eq("user"))
        .filter(message::Column::Id.gt(message_id))
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn messages_by_run<C>(db: &C, run_id: i64) -> Result<Vec<message::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    message::Entity::find()
        .filter(message::Column::RunId.eq(run_id))
        .order_by_asc(message::Column::Id)
        .all(db)
        .await
        .map_err(AgentDbError::Database)
}

/// Full unsummarized history plus the active Run's records needed for replay.
/// UI pagination must not silently discard instructions from model context.
pub async fn messages_for_context<C: ConnectionTrait>(
    db: &C,
    thread_id: i64,
    run_id: i64,
    covered_message_id: i64,
) -> Result<Vec<message::Model>, AgentDbError> {
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
        .map_err(AgentDbError::Database)
}

pub async fn turns_by_thread<C>(
    db: &C,
    thread_id: i64,
    limit: u64,
) -> Result<Vec<turn::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut rows = turn::Entity::find()
        .filter(turn::Column::ThreadId.eq(thread_id))
        .order_by_desc(turn::Column::Sequence)
        .limit(limit)
        .all(db)
        .await
        .map_err(AgentDbError::Database)?;
    rows.reverse();
    Ok(rows)
}

pub async fn runs_by_thread<C>(
    db: &C,
    thread_id: i64,
    limit: u64,
) -> Result<Vec<run::Model>, AgentDbError>
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
        .map_err(AgentDbError::Database)?;
    rows.reverse();
    Ok(rows)
}

pub async fn active_runs_by_thread<C>(
    db: &C,
    thread_id: i64,
) -> Result<Vec<run::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    Ok(run::Entity::find()
        .filter(run::Column::ThreadId.eq(thread_id))
        .order_by_desc(run::Column::UpdatedAt)
        .order_by_desc(run::Column::Id)
        .all(db)
        .await
        .map_err(AgentDbError::Database)?
        .into_iter()
        .filter(|row| is_run_active(&row.status))
        .collect())
}

pub async fn run_by_id<C>(db: &C, id: i64) -> Result<Option<run::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    run::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn executions_by_runs<C>(
    db: &C,
    run_ids: Vec<i64>,
) -> Result<Vec<execution::Model>, AgentDbError>
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
        .map_err(AgentDbError::Database)
}

pub async fn execution_by_id<C>(db: &C, id: i64) -> Result<Option<execution::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    execution::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn execution_by_key<C>(
    db: &C,
    run_id: i64,
    key: &str,
) -> Result<Option<execution::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    execution::Entity::find()
        .filter(execution::Column::RunId.eq(run_id))
        .filter(execution::Column::ExecutionKey.eq(key))
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn turn_by_id<C>(db: &C, id: i64) -> Result<Option<turn::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    turn::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn delegations_by_thread<C>(
    db: &C,
    thread_id: i64,
) -> Result<Vec<delegation::Model>, AgentDbError>
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
        .map_err(AgentDbError::Database)
}

pub async fn delegation_by_id<C>(db: &C, id: i64) -> Result<Option<delegation::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    delegation::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn delegation_by_parent_and_key<C>(
    db: &C,
    parent_run_id: i64,
    delegation_key: &str,
) -> Result<Option<delegation::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    delegation::Entity::find()
        .filter(delegation::Column::ParentRunId.eq(parent_run_id))
        .filter(delegation::Column::DelegationKey.eq(delegation_key))
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn inbox_by_thread<C>(
    db: &C,
    thread_id: i64,
) -> Result<Vec<inbox_model::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    inbox_model::Entity::find()
        .filter(inbox_model::Column::ThreadId.eq(thread_id))
        .order_by_asc(inbox_model::Column::CreatedAt)
        .all(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn inbox_by_id<C>(db: &C, id: i64) -> Result<Option<inbox_model::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    inbox_model::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn approvals_by_thread<C>(
    db: &C,
    thread_id: i64,
) -> Result<Vec<approval::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    approval::Entity::find()
        .filter(approval::Column::ThreadId.eq(thread_id))
        .order_by_asc(approval::Column::CreatedAt)
        .all(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn approval_by_id<C>(db: &C, id: i64) -> Result<Option<approval::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    approval::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}
