use a4n_db_models::models::project::{
    novel_project, project_source_snapshot, source_snapshot_chapter,
};
use a4n_domain::media::{CreateMediaAssetRecordInput, MediaAssetRecord};
use a4n_domain::project::ProjectSourceMode;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect, TransactionTrait,
};

use super::MediaDbError;
use super::provenance::insert_media_asset_with_provenance_record;

#[derive(Debug, FromQueryResult)]
struct MediaProjectIdentity {
    id: i64,
    project_kind: String,
    root_project_id: Option<i64>,
    source_snapshot_id: Option<i64>,
}

pub async fn create_media_asset_with_provenance_record<C>(
    db: &C,
    input: CreateMediaAssetRecordInput,
) -> Result<MediaAssetRecord, MediaDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    validate_media_asset_source_context_shape(&input)?;
    let tx = db.begin().await.map_err(MediaDbError::Database)?;
    validate_media_asset_project_context(&tx, &input).await?;
    let asset = insert_media_asset_with_provenance_record(&tx, input).await?;
    tx.commit().await.map_err(MediaDbError::Database)?;
    Ok(super::media_asset_record_from_model(asset))
}

pub(super) fn validate_media_asset_source_context_shape(
    input: &CreateMediaAssetRecordInput,
) -> Result<(), MediaDbError> {
    if input.source_chapter_id.is_some() != input.source_snapshot_chapter_id.is_some() {
        return Err(MediaDbError::InvalidArgument(
            "source_chapter_id 和 source_snapshot_chapter_id 必须同时提供或同时为空".to_string(),
        ));
    }
    Ok(())
}

async fn validate_media_asset_project_context<C>(
    db: &C,
    input: &CreateMediaAssetRecordInput,
) -> Result<(), MediaDbError>
where
    C: ConnectionTrait,
{
    let use_row_lock = db.get_database_backend() != DbBackend::Sqlite;
    if let Some(source_snapshot_id) = input.source_snapshot_id {
        lock_snapshot_backed_media_context_rows(db, input, source_snapshot_id, use_row_lock)
            .await?;
        crate::project::validate_project_family_source_context(
            db,
            input.project_id,
            input.root_project_id,
            source_snapshot_id,
        )
        .await
        .map_err(media_project_family_error)?;
        if let Some(source_snapshot_chapter_id) = input.source_snapshot_chapter_id {
            crate::project::validate_project_family_snapshot_chapter(
                db,
                input.root_project_id,
                source_snapshot_id,
                source_snapshot_chapter_id,
                input.source_chapter_id,
            )
            .await
            .map_err(media_project_family_error)?;
        }
        return Ok(());
    }

    let mut project_query = novel_project::Entity::find_by_id(input.project_id);
    if use_row_lock {
        project_query = project_query.lock_exclusive();
    }
    let project = project_query
        .select_only()
        .column(novel_project::Column::Id)
        .column(novel_project::Column::ProjectKind)
        .column(novel_project::Column::RootProjectId)
        .column(novel_project::Column::SourceSnapshotId)
        .into_model::<MediaProjectIdentity>()
        .one(db)
        .await
        .map_err(MediaDbError::Database)?
        .ok_or_else(|| MediaDbError::NotFound(format!("项目不存在: {}", input.project_id)))?;
    if ProjectSourceMode::from_identity(
        &project.project_kind,
        project.root_project_id,
        project.source_snapshot_id,
    )
    .map_err(|error| MediaDbError::InvalidArgument(format!("项目 {}: {error}", project.id)))?
        != ProjectSourceMode::Freeform
    {
        return Err(MediaDbError::InvalidArgument(
            "缺少 source_snapshot_id 的媒体资产只能创建在 freeform 项目中".to_string(),
        ));
    }
    if input.root_project_id != project.id {
        return Err(MediaDbError::InvalidArgument(
            "freeform 媒体资产 root_project_id 必须等于项目自身".to_string(),
        ));
    }
    if input.source_snapshot_chapter_id.is_some() {
        return Err(MediaDbError::InvalidArgument(
            "freeform 媒体资产不能设置 source_snapshot_chapter_id".to_string(),
        ));
    }
    Ok(())
}

async fn lock_snapshot_backed_media_context_rows<C>(
    db: &C,
    input: &CreateMediaAssetRecordInput,
    source_snapshot_id: i64,
    use_row_lock: bool,
) -> Result<(), MediaDbError>
where
    C: ConnectionTrait,
{
    if !use_row_lock {
        return Ok(());
    }
    let mut project_ids = vec![input.project_id, input.root_project_id];
    project_ids.sort_unstable();
    project_ids.dedup();
    novel_project::Entity::find()
        .filter(novel_project::Column::Id.is_in(project_ids))
        .order_by_asc(novel_project::Column::Id)
        .select_only()
        .column(novel_project::Column::Id)
        .lock_exclusive()
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(MediaDbError::Database)?;
    project_source_snapshot::Entity::find_by_id(source_snapshot_id)
        .select_only()
        .column(project_source_snapshot::Column::Id)
        .lock_exclusive()
        .into_tuple::<i64>()
        .one(db)
        .await
        .map_err(MediaDbError::Database)?;
    if let Some(source_snapshot_chapter_id) = input.source_snapshot_chapter_id {
        source_snapshot_chapter::Entity::find_by_id(source_snapshot_chapter_id)
            .select_only()
            .column(source_snapshot_chapter::Column::Id)
            .lock_exclusive()
            .into_tuple::<i64>()
            .one(db)
            .await
            .map_err(MediaDbError::Database)?;
    }
    Ok(())
}

fn media_project_family_error(error: crate::project::ProjectFamilyDbError) -> MediaDbError {
    match error {
        crate::project::ProjectFamilyDbError::InvalidArgument(message) => {
            MediaDbError::InvalidArgument(message)
        }
        crate::project::ProjectFamilyDbError::NotFound(message) => MediaDbError::NotFound(message),
        crate::project::ProjectFamilyDbError::Database(error) => MediaDbError::Database(error),
    }
}
