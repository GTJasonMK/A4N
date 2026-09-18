use super::*;

pub async fn optimization_sessions<C>(
    db: &C,
    project_id: i64,
    chapter_id: Option<i64>,
) -> Result<Vec<optimization_session::Model>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    let mut query = optimization_session::Entity::find()
        .filter(optimization_session::Column::ProjectId.eq(project_id))
        .filter(optimization_session::Column::ArchivedAt.is_null());
    if let Some(chapter_id) = chapter_id {
        query = query.filter(optimization_session::Column::ChapterId.eq(chapter_id));
    }
    query
        .order_by_desc(optimization_session::Column::UpdatedAt)
        .order_by_desc(optimization_session::Column::Id)
        .all(db)
        .await
        .map_err(OptimizeDbError::Database)
}

pub async fn optimization_session_by_id<C>(
    db: &C,
    session_id: i64,
) -> Result<Option<optimization_session::Model>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_session::Entity::find_by_id(session_id)
        .one(db)
        .await
        .map_err(OptimizeDbError::Database)
}

pub async fn optimization_session_name_exists<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    name: &str,
) -> Result<bool, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_session::Entity::find()
        .select_only()
        .column(optimization_session::Column::Id)
        .filter(optimization_session::Column::ProjectId.eq(project_id))
        .filter(optimization_session::Column::ChapterId.eq(chapter_id))
        .filter(optimization_session::Column::ArchivedAt.is_null())
        .filter(optimization_session::Column::Name.eq(name))
        .into_tuple::<i64>()
        .one(db)
        .await
        .map(|session| session.is_some())
        .map_err(OptimizeDbError::Database)
}

pub async fn create_optimization_session<C>(
    db: &C,
    input: OptimizationSessionCreateInput,
) -> Result<optimization_session::Model, OptimizeDbError>
where
    C: ConnectionTrait,
{
    let now = chrono::Utc::now().naive_utc();
    optimization_session::ActiveModel {
        project_id: Set(input.project_id),
        chapter_id: Set(input.chapter_id),
        name: Set(input.name),
        status: Set(input.status),
        issues: Set(input.issues),
        agent_permissions: Set(input.agent_permissions),
        agent_instruction: Set(input.agent_instruction),
        fixes_applied: Set(input.fixes_applied),
        summary: Set(input.summary),
        archived_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(OptimizeDbError::Database)
}

pub async fn update_optimization_session<C>(
    db: &C,
    input: OptimizationSessionUpdateInput,
) -> Result<optimization_session::Model, OptimizeDbError>
where
    C: ConnectionTrait,
{
    let mut active: optimization_session::ActiveModel = input.session.into();
    if let Some(status) = input.status {
        active.status = Set(status);
    }
    if let Some(issues) = input.issues {
        active.issues = Set(issues);
    }
    if let Some(agent_permissions) = input.agent_permissions {
        active.agent_permissions = Set(agent_permissions);
    }
    if let Some(agent_instruction) = input.agent_instruction {
        active.agent_instruction = Set(agent_instruction);
    }
    if let Some(fixes_applied) = input.fixes_applied {
        active.fixes_applied = Set(fixes_applied);
    }
    if let Some(summary) = input.summary {
        active.summary = Set(summary);
    }
    active.updated_at = Set(chrono::Utc::now().naive_utc());
    active.update(db).await.map_err(OptimizeDbError::Database)
}

pub async fn optimization_session_messages<C>(
    db: &C,
    session_id: i64,
) -> Result<Vec<optimization_session_message::Model>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_session_message::Entity::find()
        .filter(optimization_session_message::Column::SessionId.eq(session_id))
        .order_by_asc(optimization_session_message::Column::CreatedAt)
        .order_by_asc(optimization_session_message::Column::Id)
        .all(db)
        .await
        .map_err(OptimizeDbError::Database)
}

pub async fn insert_optimization_session_message<C>(
    db: &C,
    input: OptimizationSessionMessageInsertInput,
) -> Result<optimization_session_message::Model, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_session_message::ActiveModel {
        session_id: Set(input.session_id),
        role: Set(input.role),
        content: Set(input.content),
        tool_calls: Set(input.tool_calls),
        created_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(OptimizeDbError::Database)
}

pub async fn delete_optimization_session_with_messages<C>(
    db: &C,
    session_id: i64,
) -> Result<u64, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_session_message::Entity::delete_many()
        .filter(optimization_session_message::Column::SessionId.eq(session_id))
        .exec(db)
        .await
        .map_err(OptimizeDbError::Database)?;
    optimization_session::Entity::delete_many()
        .filter(optimization_session::Column::Id.eq(session_id))
        .exec(db)
        .await
        .map(|result| result.rows_affected)
        .map_err(OptimizeDbError::Database)
}
