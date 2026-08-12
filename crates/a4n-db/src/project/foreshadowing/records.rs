use std::collections::BTreeSet;

use a4n_core::models::character::foreshadowing_index;
use a4n_domain::character::ForeshadowingStatus;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde_json::Value;

use super::{ForeshadowingDbError, ForeshadowingRecordInput};

pub async fn foreshadowing_records_by_project(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<foreshadowing_index::Model>, ForeshadowingDbError> {
    foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .order_by_asc(foreshadowing_index::Column::SetupChapter)
        .order_by_asc(foreshadowing_index::Column::Id)
        .all(db)
        .await
        .map_err(ForeshadowingDbError::Database)
}

pub async fn create_foreshadowing_record(
    db: &DatabaseConnection,
    project_id: i64,
    input: ForeshadowingRecordInput,
) -> Result<foreshadowing_index::Model, ForeshadowingDbError> {
    let input = normalize_record_input(input)?;
    let tx = db.begin().await?;
    lock_project_for_write(&tx, project_id, "创建伏笔").await?;
    ensure_unique_description(&tx, project_id, None, &input).await?;

    let now = Utc::now().naive_utc();
    let created = foreshadowing_index::ActiveModel {
        project_id: Set(project_id),
        description: Set(input.description),
        setup_chapter: Set(input.setup_chapter),
        payoff_chapter: Set(input.payoff_chapter),
        status: Set(input.status.as_str().to_string()),
        related_characters: Set(related_characters_json(input.related_characters)),
        importance: Set(input.importance),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    tx.commit().await?;
    Ok(created)
}

pub async fn update_foreshadowing_record(
    db: &DatabaseConnection,
    project_id: i64,
    foreshadowing_id: i64,
    expected_updated_at: &str,
    input: ForeshadowingRecordInput,
) -> Result<foreshadowing_index::Model, ForeshadowingDbError> {
    let input = normalize_record_input(input)?;
    let tx = db.begin().await?;
    lock_project_for_write(&tx, project_id, "更新伏笔").await?;
    let existing = record_for_update(&tx, project_id, foreshadowing_id).await?;
    require_current_version(&existing, expected_updated_at, "保存旧伏笔表单")?;
    ensure_unique_description(&tx, project_id, Some(foreshadowing_id), &input).await?;

    let updated_at = std::cmp::max(
        Utc::now().naive_utc(),
        existing.updated_at + chrono::Duration::microseconds(1),
    );
    let mut active: foreshadowing_index::ActiveModel = existing.into();
    active.description = Set(input.description);
    active.setup_chapter = Set(input.setup_chapter);
    active.payoff_chapter = Set(input.payoff_chapter);
    active.status = Set(input.status.as_str().to_string());
    active.related_characters = Set(related_characters_json(input.related_characters));
    active.importance = Set(input.importance);
    active.updated_at = Set(updated_at);
    let updated = active.update(&tx).await?;
    tx.commit().await?;
    Ok(updated)
}

pub async fn delete_foreshadowing_record(
    db: &DatabaseConnection,
    project_id: i64,
    foreshadowing_id: i64,
    expected_updated_at: &str,
) -> Result<foreshadowing_index::Model, ForeshadowingDbError> {
    let tx = db.begin().await?;
    lock_project_for_write(&tx, project_id, "删除伏笔").await?;
    let existing = record_for_update(&tx, project_id, foreshadowing_id).await?;
    require_current_version(&existing, expected_updated_at, "删除旧伏笔记录")?;
    foreshadowing_index::Entity::delete_by_id(existing.id)
        .exec(&tx)
        .await?;
    tx.commit().await?;
    Ok(existing)
}

pub(super) async fn lock_project_for_write<C>(
    db: &C,
    project_id: i64,
    operation: &str,
) -> Result<(), ForeshadowingDbError>
where
    C: ConnectionTrait,
{
    let project = crate::project::project_by_id_for_update(db, project_id)
        .await?
        .ok_or(ForeshadowingDbError::ProjectNotFound { project_id })?;
    if project.lifecycle_gate != crate::generation::PROJECT_GATE_ACTIVE {
        return Err(ForeshadowingDbError::InvalidState {
            current: project.lifecycle_gate,
            operation: operation.to_string(),
        });
    }
    Ok(())
}

async fn record_for_update<C>(
    db: &C,
    project_id: i64,
    foreshadowing_id: i64,
) -> Result<foreshadowing_index::Model, ForeshadowingDbError>
where
    C: ConnectionTrait,
{
    let mut query = foreshadowing_index::Entity::find_by_id(foreshadowing_id)
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id));
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    query
        .one(db)
        .await?
        .ok_or(ForeshadowingDbError::RecordNotFound {
            project_id,
            foreshadowing_id,
        })
}

async fn ensure_unique_description<C>(
    db: &C,
    project_id: i64,
    excluded_id: Option<i64>,
    input: &ForeshadowingRecordInput,
) -> Result<(), ForeshadowingDbError>
where
    C: ConnectionTrait,
{
    let mut query = foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .filter(foreshadowing_index::Column::SetupChapter.eq(input.setup_chapter))
        .filter(foreshadowing_index::Column::Description.eq(input.description.clone()));
    if let Some(excluded_id) = excluded_id {
        query = query.filter(foreshadowing_index::Column::Id.ne(excluded_id));
    }
    if query.one(db).await?.is_some() {
        return Err(ForeshadowingDbError::InvalidArgument(format!(
            "第 {} 章已经存在同名伏笔：{}",
            input.setup_chapter, input.description
        )));
    }
    Ok(())
}

fn require_current_version(
    existing: &foreshadowing_index::Model,
    expected_updated_at: &str,
    operation: &str,
) -> Result<(), ForeshadowingDbError> {
    if expected_updated_at.trim().is_empty() {
        return Err(ForeshadowingDbError::InvalidArgument(
            "缺少伏笔资料版本，请刷新后重试".to_string(),
        ));
    }
    if existing.updated_at.to_string() != expected_updated_at.trim() {
        return Err(ForeshadowingDbError::InvalidState {
            current: "foreshadowing_source_changed".to_string(),
            operation: format!("{operation}；伏笔资料已经变化，请刷新后重新编辑"),
        });
    }
    Ok(())
}

fn normalize_record_input(
    input: ForeshadowingRecordInput,
) -> Result<ForeshadowingRecordInput, ForeshadowingDbError> {
    let description = input.description.trim().to_string();
    if description.is_empty() {
        return Err(ForeshadowingDbError::InvalidArgument(
            "伏笔描述不能为空".to_string(),
        ));
    }
    if input.setup_chapter <= 0 {
        return Err(ForeshadowingDbError::InvalidArgument(
            "埋设章节必须大于 0".to_string(),
        ));
    }
    if let Some(payoff_chapter) = input.payoff_chapter
        && payoff_chapter <= input.setup_chapter
    {
        return Err(ForeshadowingDbError::InvalidArgument(
            "回收章节必须晚于埋设章节".to_string(),
        ));
    }
    if (input.status == ForeshadowingStatus::Resolved) != input.payoff_chapter.is_some() {
        return Err(ForeshadowingDbError::InvalidArgument(
            "已回收伏笔必须填写回收章节，其他状态不能填写回收章节".to_string(),
        ));
    }
    if !(1..=5).contains(&input.importance) {
        return Err(ForeshadowingDbError::InvalidArgument(
            "伏笔重要度必须在 1 到 5 之间".to_string(),
        ));
    }

    let mut seen = BTreeSet::new();
    let related_characters = input
        .related_characters
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && seen.insert(value.clone()))
        .collect();
    Ok(ForeshadowingRecordInput {
        description,
        setup_chapter: input.setup_chapter,
        payoff_chapter: input.payoff_chapter,
        status: input.status,
        related_characters,
        importance: input.importance,
    })
}

fn related_characters_json(related_characters: Vec<String>) -> Option<Value> {
    (!related_characters.is_empty()).then(|| serde_json::json!(related_characters))
}
