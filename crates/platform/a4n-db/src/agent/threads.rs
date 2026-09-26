use super::*;

pub async fn lock_thread<C>(db: &C, thread_id: i64) -> Result<Option<thread::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    thread::Entity::find_by_id(thread_id)
        .lock_exclusive()
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn update_thread_record<C>(
    db: &C,
    row: thread::Model,
    scope_json: Option<serde_json::Value>,
    title: Option<Option<String>>,
    default_model_config_id: Option<i64>,
    revision: i64,
) -> Result<thread::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut active: thread::ActiveModel = row.into();
    if let Some(scope_json) = scope_json {
        active.scope_json = Set(scope_json);
    }
    if let Some(title) = title {
        active.title = Set(title);
    }
    if let Some(config_id) = default_model_config_id {
        active.default_model_config_id = Set(Some(config_id));
    }
    active.revision = Set(revision);
    active.updated_at = Set(now());
    active.update(db).await.map_err(AgentDbError::Database)
}

pub async fn update_thread_scope<C>(
    db: &C,
    row: thread::Model,
    scope_json: serde_json::Value,
) -> Result<thread::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut active: thread::ActiveModel = row.into();
    active.scope_json = Set(scope_json);
    active.updated_at = Set(now());
    active.update(db).await.map_err(AgentDbError::Database)
}

pub async fn archive_thread_record<C>(
    db: &C,
    row: thread::Model,
    revision: i64,
) -> Result<thread::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut active: thread::ActiveModel = row.into();
    active.status = Set("archived".into());
    active.archived_at = Set(Some(now()));
    active.revision = Set(revision);
    active.updated_at = Set(now());
    active.update(db).await.map_err(AgentDbError::Database)
}
