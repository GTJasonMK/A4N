use a4n_db_models::models::{
    galgame::galgame_story_runtime_state as runtime_row, project::novel_project,
};
use a4n_domain::galgame::StoryRuntimeState;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, Set,
    TransactionTrait,
};

use super::types::GalgameStoryRuntimeRecordError;

#[derive(Debug, Clone)]
pub struct GalgameStoryRuntimeRecord {
    pub id: i64,
    pub user_id: i64,
    pub project_id: i64,
    pub save_slot: String,
    pub state: StoryRuntimeState,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

pub async fn galgame_story_runtime_state<C>(
    db: &C,
    user_id: i64,
    project_id: i64,
    save_slot: &str,
) -> Result<Option<GalgameStoryRuntimeRecord>, GalgameStoryRuntimeRecordError>
where
    C: ConnectionTrait,
{
    validate_identity(user_id, project_id)?;
    let save_slot = validate_save_slot(save_slot)?;
    let row = find_save(db, user_id, project_id, save_slot).await?;
    row.map(map_record).transpose()
}

pub async fn delete_galgame_story_runtime_states_for_project<C>(
    db: &C,
    project_id: i64,
) -> Result<u64, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    Ok(runtime_row::Entity::delete_many()
        .filter(runtime_row::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?
        .rows_affected)
}

pub async fn create_galgame_story_runtime_state<C>(
    db: &C,
    user_id: i64,
    project_id: i64,
    save_slot: &str,
    state: StoryRuntimeState,
) -> Result<GalgameStoryRuntimeRecord, GalgameStoryRuntimeRecordError>
where
    C: ConnectionTrait + TransactionTrait,
{
    validate_identity(user_id, project_id)?;
    let save_slot = validate_save_slot(save_slot)?;
    validate_state(&state)?;
    let tx = db.begin().await?;
    let project = require_galgame_project(&tx, project_id).await?;
    require_current_graph(&tx, &project, &state).await?;
    if find_save(&tx, user_id, project_id, save_slot)
        .await?
        .is_some()
    {
        return Err(GalgameStoryRuntimeRecordError::Conflict(format!(
            "存档槽已存在: {save_slot}"
        )));
    }
    let now = Utc::now().naive_utc();
    let row = runtime_row::ActiveModel {
        user_id: Set(user_id),
        project_id: Set(project_id),
        save_slot: Set(save_slot.to_string()),
        graph_id: Set(state.graph_id.clone()),
        graph_revision: Set(state.graph_revision),
        state_json: Set(serde_json::to_value(state).map_err(|error| {
            GalgameStoryRuntimeRecordError::InvalidArgument(format!(
                "序列化 Galgame 运行状态失败: {error}"
            ))
        })?),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    tx.commit().await?;
    map_record(row)
}

pub async fn advance_galgame_story_runtime_state<C>(
    db: &C,
    user_id: i64,
    project_id: i64,
    save_slot: &str,
    expected_step_index: u32,
    state: StoryRuntimeState,
) -> Result<GalgameStoryRuntimeRecord, GalgameStoryRuntimeRecordError>
where
    C: ConnectionTrait + TransactionTrait,
{
    validate_identity(user_id, project_id)?;
    let save_slot = validate_save_slot(save_slot)?;
    validate_state(&state)?;
    if state.step_index != expected_step_index.saturating_add(1) {
        return Err(GalgameStoryRuntimeRecordError::InvalidArgument(
            "运行状态必须只推进一个节点".into(),
        ));
    }
    let tx = db.begin().await?;
    let project = require_galgame_project(&tx, project_id).await?;
    require_current_graph(&tx, &project, &state).await?;
    let existing = find_save(&tx, user_id, project_id, save_slot)
        .await?
        .ok_or_else(|| {
            GalgameStoryRuntimeRecordError::NotFound(format!("存档槽不存在: {save_slot}"))
        })?;
    let current = map_record(existing.clone())?;
    if current.state.step_index != expected_step_index {
        return Err(GalgameStoryRuntimeRecordError::Conflict(format!(
            "运行步数冲突：expected={expected_step_index}, current={}",
            current.state.step_index
        )));
    }
    if current.state.graph_id != state.graph_id
        || current.state.graph_revision != state.graph_revision
    {
        return Err(GalgameStoryRuntimeRecordError::Conflict(
            "存档绑定的节点图版本已变化".into(),
        ));
    }
    let mut active: runtime_row::ActiveModel = existing.into();
    active.state_json = Set(serde_json::to_value(&state).map_err(|error| {
        GalgameStoryRuntimeRecordError::InvalidArgument(format!(
            "序列化 Galgame 运行状态失败: {error}"
        ))
    })?);
    active.updated_at = Set(Utc::now().naive_utc());
    let row = active.update(&tx).await?;
    tx.commit().await?;
    map_record(row)
}

async fn require_galgame_project<C>(
    db: &C,
    project_id: i64,
) -> Result<novel_project::Model, GalgameStoryRuntimeRecordError>
where
    C: ConnectionTrait,
{
    let project = novel_project::Entity::find_by_id(project_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| {
            GalgameStoryRuntimeRecordError::NotFound(format!("项目不存在: {project_id}"))
        })?;
    if project.project_kind != "galgame" {
        return Err(GalgameStoryRuntimeRecordError::InvalidArgument(
            "运行存档只属于 Galgame 项目".into(),
        ));
    }
    Ok(project)
}

async fn require_current_graph<C>(
    db: &C,
    project: &novel_project::Model,
    state: &StoryRuntimeState,
) -> Result<(), GalgameStoryRuntimeRecordError>
where
    C: ConnectionTrait,
{
    let graph = super::story_graph::current_galgame_story_graph(db, project.id)
        .await?
        .ok_or_else(|| GalgameStoryRuntimeRecordError::NotFound("Galgame 项目尚无节点图".into()))?;
    if project.source_snapshot_id != Some(graph.source_snapshot_id)
        || graph.graph.id != state.graph_id
        || graph.revision != state.graph_revision
    {
        return Err(GalgameStoryRuntimeRecordError::Conflict(
            "节点图已变化，请重新开始试玩".into(),
        ));
    }
    Ok(())
}

async fn find_save<C>(
    db: &C,
    user_id: i64,
    project_id: i64,
    save_slot: &str,
) -> Result<Option<runtime_row::Model>, GalgameStoryRuntimeRecordError>
where
    C: ConnectionTrait,
{
    Ok(runtime_row::Entity::find()
        .filter(runtime_row::Column::UserId.eq(user_id))
        .filter(runtime_row::Column::ProjectId.eq(project_id))
        .filter(runtime_row::Column::SaveSlot.eq(save_slot))
        .one(db)
        .await?)
}

fn validate_identity(user_id: i64, project_id: i64) -> Result<(), GalgameStoryRuntimeRecordError> {
    if user_id <= 0 || project_id <= 0 {
        return Err(GalgameStoryRuntimeRecordError::InvalidArgument(
            "user_id 和 project_id 必须为正整数".into(),
        ));
    }
    Ok(())
}

fn validate_save_slot(save_slot: &str) -> Result<&str, GalgameStoryRuntimeRecordError> {
    let save_slot = save_slot.trim();
    if save_slot.is_empty() || save_slot.len() > 80 {
        return Err(GalgameStoryRuntimeRecordError::InvalidArgument(
            "save_slot 长度必须为 1 到 80 个字节".into(),
        ));
    }
    Ok(save_slot)
}

fn validate_state(state: &StoryRuntimeState) -> Result<(), GalgameStoryRuntimeRecordError> {
    if state.graph_id.trim().is_empty() || state.graph_revision <= 0 {
        return Err(GalgameStoryRuntimeRecordError::InvalidArgument(
            "运行状态必须绑定有效的节点图 revision".into(),
        ));
    }
    Ok(())
}

fn map_record(
    row: runtime_row::Model,
) -> Result<GalgameStoryRuntimeRecord, GalgameStoryRuntimeRecordError> {
    let state: StoryRuntimeState = serde_json::from_value(row.state_json).map_err(|error| {
        GalgameStoryRuntimeRecordError::InvalidArgument(format!(
            "Galgame 运行状态 JSON 无法解析: {error}"
        ))
    })?;
    if state.graph_id != row.graph_id || state.graph_revision != row.graph_revision {
        return Err(GalgameStoryRuntimeRecordError::InvalidArgument(
            "运行存档图身份与状态快照不一致".into(),
        ));
    }
    Ok(GalgameStoryRuntimeRecord {
        id: row.id,
        user_id: row.user_id,
        project_id: row.project_id,
        save_slot: row.save_slot,
        state,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}
