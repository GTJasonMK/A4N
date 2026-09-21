use crate::source_sync::{SourceSyncDbError, validate_positive_id};
use a4n_db_models::models::{
    generation::generation_spec,
    project::{project_source_snapshot, source_snapshot_chapter},
    source_sync::source_sync_run,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, Set,
};
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
    let (snapshot_id, source_version_hash) =
        project_source_snapshot::Entity::find_by_id(to_snapshot_id)
            .select_only()
            .column(project_source_snapshot::Column::Id)
            .column(project_source_snapshot::Column::SourceVersionHash)
            .into_tuple::<(i64, String)>()
            .one(db)
            .await?
            .ok_or_else(|| {
                SourceSyncDbError::NotFound(format!("目标源快照不存在: {to_snapshot_id}"))
            })?;
    let source_snapshot_chapter_id = match spec.source_chapter_id {
        Some(source_chapter_id) => Some(
            source_snapshot_chapter_id_for_source(db, to_snapshot_id, source_chapter_id).await?,
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
    active.source_snapshot_id = Set(Some(snapshot_id));
    active.source_version_hash = Set(source_version_hash);
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

pub(in crate::source_sync) async fn generation_spec_id_for_source_sync_run<C>(
    db: &C,
    run: &source_sync_run::Model,
    generation_spec_id: i64,
) -> Result<i64, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(run.derived_project_id, "derived_project_id")?;
    validate_positive_id(run.root_project_id, "root_project_id")?;
    validate_positive_id(generation_spec_id, "generation_spec_id")?;
    let Some((id, project_id, root_project_id)) =
        generation_spec::Entity::find_by_id(generation_spec_id)
            .select_only()
            .column(generation_spec::Column::Id)
            .column(generation_spec::Column::ProjectId)
            .column(generation_spec::Column::RootProjectId)
            .into_tuple::<(i64, i64, i64)>()
            .one(db)
            .await?
    else {
        return Err(SourceSyncDbError::NotFound(format!(
            "生成意图不存在: {generation_spec_id}"
        )));
    };
    validate_generation_spec_owner(run, id, project_id, root_project_id)?;
    Ok(id)
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

pub(in crate::source_sync) async fn source_snapshot_chapter_id_for_source<C>(
    db: &C,
    snapshot_id: i64,
    source_chapter_id: i64,
) -> Result<i64, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(snapshot_id, "snapshot_id")?;
    validate_positive_id(source_chapter_id, "source_chapter_id")?;
    source_snapshot_chapter::Entity::find()
        .select_only()
        .column(source_snapshot_chapter::Column::Id)
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .filter(source_snapshot_chapter::Column::SourceChapterId.eq(source_chapter_id))
        .into_tuple::<i64>()
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
    validate_generation_spec_owner(run, spec.id, spec.project_id, spec.root_project_id)?;
    Ok(spec)
}

fn validate_generation_spec_owner(
    run: &source_sync_run::Model,
    generation_spec_id: i64,
    project_id: i64,
    root_project_id: i64,
) -> Result<(), SourceSyncDbError> {
    if project_id != run.derived_project_id || root_project_id != run.root_project_id {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "生成意图 {} 不属于本次源同步运行",
            generation_spec_id
        )));
    }
    Ok(())
}
