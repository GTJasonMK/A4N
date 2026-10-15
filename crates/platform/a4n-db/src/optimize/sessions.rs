use super::*;

fn optimization_session_record(
    session: optimization_session::Model,
) -> a4n_domain::project::OptimizationSessionRecord {
    a4n_domain::project::OptimizationSessionRecord {
        id: session.id,
        project_id: session.project_id,
        chapter_id: session.chapter_id,
        name: session.name,
        status: session.status,
        issues: session.issues,
        agent_permissions: session.agent_permissions,
        agent_instruction: session.agent_instruction,
        fixes_applied: session.fixes_applied,
        summary: session.summary,
        archived_at: session.archived_at,
        created_at: session.created_at,
        updated_at: session.updated_at,
    }
}

fn optimization_session_model(
    session: a4n_domain::project::OptimizationSessionRecord,
) -> optimization_session::Model {
    optimization_session::Model {
        id: session.id,
        project_id: session.project_id,
        chapter_id: session.chapter_id,
        name: session.name,
        status: session.status,
        issues: session.issues,
        agent_permissions: session.agent_permissions,
        agent_instruction: session.agent_instruction,
        fixes_applied: session.fixes_applied,
        summary: session.summary,
        archived_at: session.archived_at,
        created_at: session.created_at,
        updated_at: session.updated_at,
    }
}

pub async fn optimization_session_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: Option<i64>,
) -> Result<Vec<a4n_domain::project::OptimizationSessionRecord>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_sessions(db, project_id, chapter_id)
        .await
        .map(|sessions| {
            sessions
                .into_iter()
                .map(optimization_session_record)
                .collect()
        })
}

pub async fn optimization_session_record_by_id<C>(
    db: &C,
    session_id: i64,
) -> Result<Option<a4n_domain::project::OptimizationSessionRecord>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_session_by_id(db, session_id)
        .await
        .map(|session| session.map(optimization_session_record))
}

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

pub async fn resume_optimization_analysis_session(
    db: &impl crate::DbExecutor,
    session_id: i64,
) -> Result<optimization_session::Model, OptimizeDbError> {
    // A generation retry reuses the prepared inputs; it must not overwrite a
    // concurrent edit/fix or revive an archived session.
    optimization_session::Entity::update_many()
        .col_expr(
            optimization_session::Column::Status,
            sea_orm::sea_query::Expr::value("analyzing"),
        )
        .col_expr(
            optimization_session::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(chrono::Utc::now().naive_utc()),
        )
        .filter(optimization_session::Column::Id.eq(session_id))
        .filter(optimization_session::Column::ArchivedAt.is_null())
        .filter(optimization_session::Column::Status.is_in(["waiting_user", "cancelled"]))
        .exec_with_returning(db)
        .await
        .map_err(OptimizeDbError::Database)?
        .pop()
        .ok_or_else(|| {
            OptimizeDbError::InvalidArgument("优化分析会话状态已改变，不能恢复该次分析".into())
        })
}

pub async fn resume_optimization_analysis_session_record(
    db: &impl crate::DbExecutor,
    session_id: i64,
) -> Result<a4n_domain::project::OptimizationSessionRecord, OptimizeDbError> {
    resume_optimization_analysis_session(db, session_id)
        .await
        .map(optimization_session_record)
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

pub async fn create_optimization_session(
    db: &impl crate::DbExecutor,
    input: OptimizationSessionCreateInput,
) -> Result<optimization_session::Model, OptimizeDbError> {
    let tx = db.begin().await?;
    crate::generation::require_project_owner_lifecycle_active(&tx, input.project_id).await?;
    let now = chrono::Utc::now().naive_utc();
    let session = optimization_session::ActiveModel {
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
    .insert(&tx)
    .await?;
    tx.commit().await?;
    Ok(session)
}

pub async fn create_optimization_session_record(
    db: &impl crate::DbExecutor,
    input: OptimizationSessionCreateInput,
) -> Result<a4n_domain::project::OptimizationSessionRecord, OptimizeDbError> {
    create_optimization_session(db, input)
        .await
        .map(optimization_session_record)
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

pub async fn update_optimization_session_record<C>(
    db: &C,
    input: OptimizationSessionRecordUpdateInput,
) -> Result<a4n_domain::project::OptimizationSessionRecord, OptimizeDbError>
where
    C: ConnectionTrait,
{
    update_optimization_session(
        db,
        OptimizationSessionUpdateInput {
            session: optimization_session_model(input.session),
            status: input.status,
            issues: input.issues,
            agent_permissions: input.agent_permissions,
            agent_instruction: input.agent_instruction,
            fixes_applied: input.fixes_applied,
            summary: input.summary,
        },
    )
    .await
    .map(optimization_session_record)
}

pub async fn optimization_session_messages<C>(
    db: &C,
    session_id: i64,
) -> Result<Vec<a4n_domain::project::OptimizationSessionMessageRecord>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_session_message::Entity::find()
        .filter(optimization_session_message::Column::SessionId.eq(session_id))
        .order_by_asc(optimization_session_message::Column::CreatedAt)
        .order_by_asc(optimization_session_message::Column::Id)
        .all(db)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(optimization_session_message_record)
                .collect()
        })
        .map_err(OptimizeDbError::Database)
}

pub async fn insert_optimization_session_message<C>(
    db: &C,
    input: OptimizationSessionMessageInsertInput,
) -> Result<a4n_domain::project::OptimizationSessionMessageRecord, OptimizeDbError>
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
    .map(optimization_session_message_record)
    .map_err(OptimizeDbError::Database)
}

fn optimization_session_message_record(
    message: optimization_session_message::Model,
) -> a4n_domain::project::OptimizationSessionMessageRecord {
    a4n_domain::project::OptimizationSessionMessageRecord {
        id: message.id,
        session_id: message.session_id,
        role: message.role,
        content: message.content,
        tool_calls: message.tool_calls,
        created_at: message.created_at,
    }
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
