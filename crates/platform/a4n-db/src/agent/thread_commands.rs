use super::*;
use sea_orm::{AccessMode, ActiveModelTrait, EntityTrait, IsolationLevel, QuerySelect, Set};

pub async fn lock_thread<C: sea_orm::ConnectionTrait>(
    db: &C,
    thread_id: i64,
) -> Result<AgentThreadRecord, AgentDbError> {
    thread::Entity::find_by_id(thread_id)
        .lock_exclusive()
        .one(db)
        .await?
        .map(AgentThreadRecord::from)
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent thread 不存在: {thread_id}")))
}

pub async fn update_thread_record<C: sea_orm::ConnectionTrait>(
    db: &C,
    input: AgentThreadUpdateRecordInput,
) -> Result<AgentThreadRecord, AgentDbError> {
    let model = thread::Entity::find_by_id(input.thread_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", input.thread_id))
        })?;
    if model.owner_user_id != input.owner_user_id {
        return Err(AgentDbError::Forbidden(
            "不能修改其他用户的 Agent thread".into(),
        ));
    }
    if model.revision != input.expected_revision {
        return Err(AgentDbError::Conflict(format!(
            "Agent thread revision 冲突: expected={}, current={}",
            input.expected_revision, model.revision
        )));
    }
    if input.scope_json.is_some() && !active_runs_by_thread(db, input.thread_id).await?.is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "Agent thread 存在活动 Run，不能修改工具权限".into(),
        ));
    }

    let mut active: thread::ActiveModel = model.into();
    if let Some(scope) = input.scope_json {
        active.scope_json = Set(scope);
    }
    if let Some(title) = input.title {
        active.title = Set(title);
    }
    if let Some(model_config_id) = input.default_model_config_id {
        active.default_model_config_id = Set(Some(model_config_id));
    }
    active.revision = Set(input
        .expected_revision
        .checked_add(1)
        .ok_or_else(|| AgentDbError::InvalidArgument("Agent thread revision 溢出".into()))?);
    active.updated_at = Set(now());
    active
        .update(db)
        .await
        .map(AgentThreadRecord::from)
        .map_err(AgentDbError::from)
}

pub async fn archive_thread_record<C: sea_orm::ConnectionTrait>(
    db: &C,
    thread_id: i64,
    owner_user_id: i64,
) -> Result<AgentThreadRecord, AgentDbError> {
    let model = thread::Entity::find_by_id(thread_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent thread 不存在: {thread_id}")))?;
    if model.owner_user_id != owner_user_id {
        return Err(AgentDbError::Forbidden("无权访问此 Agent thread".into()));
    }
    if !active_runs_by_thread(db, thread_id).await?.is_empty() {
        return Err(AgentDbError::InvalidState {
            current: "running".into(),
            operation: "归档 Agent thread".into(),
        });
    }
    let next_revision = model
        .revision
        .checked_add(1)
        .ok_or_else(|| AgentDbError::InvalidArgument("Agent thread revision 溢出".into()))?;
    let mut active: thread::ActiveModel = model.into();
    active.status = Set("archived".into());
    active.archived_at = Set(Some(now()));
    active.revision = Set(next_revision);
    active.updated_at = Set(now());
    active
        .update(db)
        .await
        .map(AgentThreadRecord::from)
        .map_err(AgentDbError::from)
}

pub async fn reconcile_thread_scope_record<C: sea_orm::ConnectionTrait>(
    db: &C,
    thread_id: i64,
    capability: &str,
    expected_kind: &str,
    expected_revision: i64,
    expected_scope_json: &Value,
    scope_json: Value,
) -> Result<AgentThreadRecord, AgentDbError> {
    let model = thread::Entity::find_by_id(thread_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent thread 不存在: {thread_id}")))?;
    if model.thread_kind != expected_kind || model.profile_id != capability {
        return Err(AgentDbError::Conflict(format!(
            "canonical Agent thread {thread_id} 配置不一致"
        )));
    }
    if model.revision != expected_revision || &model.scope_json != expected_scope_json {
        return Err(AgentDbError::Conflict(format!(
            "canonical Agent thread {thread_id} 已被并发修改"
        )));
    }
    let mut active: thread::ActiveModel = model.into();
    active.scope_json = Set(scope_json);
    active.updated_at = Set(now());
    active
        .update(db)
        .await
        .map(AgentThreadRecord::from)
        .map_err(AgentDbError::from)
}

pub async fn thread_history(
    db: &impl crate::DbExecutor,
    owner_user_id: i64,
    thread_id: i64,
    limit: u64,
) -> Result<AgentThreadHistoryRecord, AgentDbError> {
    let tx = db
        .begin_with_config(
            Some(IsolationLevel::RepeatableRead),
            Some(AccessMode::ReadOnly),
        )
        .await?;
    let thread = thread_by_id(&tx, thread_id)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent thread 不存在: {thread_id}")))?;
    let messages = messages_by_thread(&tx, thread_id, None, limit).await?;
    let turns = turns_by_thread(&tx, thread_id, limit).await?;
    let mut runs = runs_by_thread(&tx, thread_id, limit).await?;
    let executions = executions_by_runs(&tx, runs.iter().map(|row| row.id).collect()).await?;
    let delegations = delegations_by_thread(&tx, thread_id).await?;
    let approvals = approvals_by_thread(&tx, thread_id).await?;
    let inbox = inbox_by_thread(&tx, thread_id).await?;
    let memories = list_memory(
        &tx,
        owner_user_id,
        Some("thread"),
        Some(&thread_id.to_string()),
    )
    .await?;
    let events = latest_events_by_thread(&tx, thread_id, limit).await?;
    for run in &mut runs {
        run.usage_json = run_budget_snapshot(&tx, run.id).await?.usage;
    }
    tx.commit().await?;
    Ok(AgentThreadHistoryRecord {
        thread,
        messages,
        turns,
        runs,
        executions,
        delegations,
        approvals,
        inbox,
        memories,
        events,
    })
}
