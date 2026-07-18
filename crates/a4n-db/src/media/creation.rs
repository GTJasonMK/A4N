use a4n_core::models::{
    media::media_asset,
    project::{novel_project, project_source_snapshot, source_snapshot_chapter},
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};

use super::provenance::insert_media_asset_with_provenance_record;
use super::{CreateMediaAssetRecordInput, MediaDbError};

pub async fn create_media_asset_with_provenance_record<C>(
    db: &C,
    input: CreateMediaAssetRecordInput,
) -> Result<media_asset::Model, MediaDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    validate_media_asset_source_context_shape(&input)?;
    let tx = db.begin().await.map_err(MediaDbError::Database)?;
    validate_media_asset_project_context(&tx, &input).await?;
    let asset = insert_media_asset_with_provenance_record(&tx, input).await?;
    tx.commit().await.map_err(MediaDbError::Database)?;
    Ok(asset)
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
        let context = crate::project::project_family_context(
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
                &context,
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
        .one(db)
        .await
        .map_err(MediaDbError::Database)?
        .ok_or_else(|| MediaDbError::NotFound(format!("项目不存在: {}", input.project_id)))?;
    if project.source_mode != crate::project::SOURCE_MODE_FREEFORM {
        return Err(MediaDbError::InvalidArgument(
            "缺少 source_snapshot_id 的媒体资产只能创建在 freeform 项目中".to_string(),
        ));
    }
    if input.root_project_id != project.id {
        return Err(MediaDbError::InvalidArgument(
            "freeform 媒体资产 root_project_id 必须等于项目自身".to_string(),
        ));
    }
    if project.root_project_id.is_some()
        || project.parent_project_id.is_some()
        || project.source_snapshot_id.is_some()
    {
        return Err(MediaDbError::InvalidArgument(
            "freeform 媒体资产项目不能包含文本派生上下文字段".to_string(),
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
        .lock_exclusive()
        .all(db)
        .await
        .map_err(MediaDbError::Database)?;
    project_source_snapshot::Entity::find_by_id(source_snapshot_id)
        .lock_exclusive()
        .one(db)
        .await
        .map_err(MediaDbError::Database)?;
    if let Some(source_snapshot_chapter_id) = input.source_snapshot_chapter_id {
        source_snapshot_chapter::Entity::find_by_id(source_snapshot_chapter_id)
            .lock_exclusive()
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
