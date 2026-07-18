use crate::source_sync::{SourceSyncDbError, validate_positive_id};
use a4n_core::models::{
    generation::generation_spec,
    project::{project_source_snapshot, source_snapshot_chapter},
    source_sync::source_sync_run,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use serde_json::Value;

pub async fn update_generation_spec_to_snapshot_record<C>(
    db: &C,
    spec: &generation_spec::Model,
    to_snapshot_id: i64,
) -> Result<generation_spec::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(spec.id, "generation_spec_id")?;
    validate_positive_id(to_snapshot_id, "to_snapshot_id")?;
    let snapshot = project_source_snapshot::Entity::find_by_id(to_snapshot_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            SourceSyncDbError::NotFound(format!("目标源快照不存在: {to_snapshot_id}"))
        })?;
    let source_snapshot_chapter_id = match spec.source_chapter_id {
        Some(source_chapter_id) => Some(
            source_snapshot_chapter_for_source(db, to_snapshot_id, source_chapter_id)
                .await?
                .id,
        ),
        None => None,
    };
    let target_entity_id = if spec.target_entity_kind == "chapter" {
        source_snapshot_chapter_id
    } else {
        spec.target_entity_id
    };
    let mut active: generation_spec::ActiveModel = spec.clone().into();
    active.source_context_kind = Set(Some("snapshot_backed".to_string()));
    active.source_snapshot_id = Set(Some(snapshot.id));
    active.source_version_hash = Set(snapshot.source_version_hash);
    active.source_snapshot_chapter_id = Set(source_snapshot_chapter_id);
    active.target_entity_id = Set(target_entity_id);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(SourceSyncDbError::Database)
}

pub async fn generation_spec_for_source_sync_run<C>(
    db: &C,
    run: &source_sync_run::Model,
    generation_spec_id: i64,
) -> Result<generation_spec::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(run.derived_project_id, "derived_project_id")?;
    validate_positive_id(run.root_project_id, "root_project_id")?;
    validate_positive_id(generation_spec_id, "generation_spec_id")?;
    generation_spec_for_run(db, run, generation_spec_id).await
}

pub async fn update_generation_spec_params_record<C>(
    db: &C,
    spec: generation_spec::Model,
    params_json: Value,
) -> Result<generation_spec::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(spec.id, "generation_spec_id")?;
    if !params_json.is_object() {
        return Err(SourceSyncDbError::InvalidArgument(
            "params_json 必须是 JSON 对象".to_string(),
        ));
    }
    let mut active: generation_spec::ActiveModel = spec.into();
    active.params_json = Set(params_json);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(SourceSyncDbError::Database)
}

pub(in crate::source_sync) async fn source_snapshot_chapter_for_source<C>(
    db: &C,
    snapshot_id: i64,
    source_chapter_id: i64,
) -> Result<source_snapshot_chapter::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(snapshot_id, "snapshot_id")?;
    validate_positive_id(source_chapter_id, "source_chapter_id")?;
    source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .filter(source_snapshot_chapter::Column::SourceChapterId.eq(source_chapter_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            SourceSyncDbError::InvalidArgument(format!(
                "目标源快照 {snapshot_id} 中不存在源章节 {source_chapter_id}"
            ))
        })
}

pub(super) async fn generation_spec_for_run<C>(
    db: &C,
    run: &source_sync_run::Model,
    generation_spec_id: i64,
) -> Result<generation_spec::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let spec = generation_spec::Entity::find_by_id(generation_spec_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            SourceSyncDbError::NotFound(format!("生成意图不存在: {generation_spec_id}"))
        })?;
    if spec.project_id != run.derived_project_id || spec.root_project_id != run.root_project_id {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "生成意图 {} 不属于本次源同步运行",
            generation_spec_id
        )));
    }
    Ok(spec)
}
