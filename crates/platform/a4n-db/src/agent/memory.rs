use super::*;
use a4n_db_models::models::agent::memory as memory_model;

pub async fn insert_memory<C>(
    db: &C,
    input: AgentMemoryInput,
) -> Result<memory_model::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    if input.revision <= 0 {
        return Err(AgentDbError::InvalidArgument(
            "memory revision 必须为正数".into(),
        ));
    }
    let timestamp = now();
    memory_model::ActiveModel {
        owner_user_id: Set(input.owner_user_id),
        project_id: Set(input.project_id),
        thread_id: Set(input.thread_id),
        scope_kind: Set(input.scope_kind),
        scope_key: Set(input.scope_key),
        memory_key: Set(input.memory_key),
        revision: Set(input.revision),
        status: Set(input.status),
        content: Set(input.content),
        content_json: Set(input.content_json),
        source_message_id: Set(input.source_message_id),
        source_execution_id: Set(input.source_execution_id),
        supersedes_memory_id: Set(input.supersedes_memory_id),
        created_at: Set(timestamp),
        updated_at: Set(timestamp),
        retracted_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AgentDbError::Database)
}

pub async fn list_memory<C>(
    db: &C,
    owner_user_id: i64,
    scope_kind: Option<&str>,
    scope_key: Option<&str>,
) -> Result<Vec<memory_model::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut query = memory_model::Entity::find()
        .filter(memory_model::Column::OwnerUserId.eq(owner_user_id))
        .order_by_desc(memory_model::Column::CreatedAt);
    if let Some(kind) = scope_kind {
        query = query.filter(memory_model::Column::ScopeKind.eq(kind));
    }
    if let Some(key) = scope_key {
        query = query.filter(memory_model::Column::ScopeKey.eq(key));
    }
    query.all(db).await.map_err(AgentDbError::Database)
}

pub async fn retract_memory<C>(
    db: &C,
    owner_user_id: i64,
    memory_id: i64,
) -> Result<memory_model::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    let row = memory_model::Entity::find_by_id(memory_id)
        .one(db)
        .await
        .map_err(AgentDbError::Database)?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent memory 不存在: {memory_id}")))?;
    if row.owner_user_id != owner_user_id {
        return Err(AgentDbError::Forbidden("无权修改此 Agent memory".into()));
    }

    let timestamp = now();
    let mut active: memory_model::ActiveModel = row.into();
    active.status = Set("retracted".into());
    active.retracted_at = Set(Some(timestamp));
    active.updated_at = Set(timestamp);
    active.update(db).await.map_err(AgentDbError::Database)
}
