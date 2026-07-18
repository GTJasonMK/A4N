use crate::canvas::CanvasDbError;
use a4n_core::models::{
    ai::{generated_image, generated_video},
    media::media_asset,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};

pub async fn canvas_media_asset_by_id<C>(
    db: &C,
    media_asset_id: i64,
) -> Result<Option<media_asset::Model>, CanvasDbError>
where
    C: ConnectionTrait,
{
    Ok(media_asset::Entity::find_by_id(media_asset_id)
        .one(db)
        .await?)
}

pub(super) async fn canvas_generated_image_for_media_ensure<C>(
    db: &C,
    project_id: i64,
    image_id: i64,
) -> Result<generated_image::Model, CanvasDbError>
where
    C: ConnectionTrait,
{
    let mut query = generated_image::Entity::find_by_id(image_id);
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let image = query
        .one(db)
        .await?
        .ok_or_else(|| CanvasDbError::NotFound(format!("图片不存在: {image_id}")))?;
    if image.project_id != project_id {
        return Err(CanvasDbError::InvalidArgument(format!(
            "图片不属于当前项目: {}",
            image.id
        )));
    }
    Ok(image)
}

pub(super) async fn canvas_generated_video_for_media_ensure<C>(
    db: &C,
    project_id: i64,
    video_id: i64,
) -> Result<generated_video::Model, CanvasDbError>
where
    C: ConnectionTrait,
{
    let mut query = generated_video::Entity::find_by_id(video_id);
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let video = query
        .one(db)
        .await?
        .ok_or_else(|| CanvasDbError::NotFound(format!("视频不存在: {video_id}")))?;
    if video.project_id != project_id {
        return Err(CanvasDbError::InvalidArgument(format!(
            "视频不属于当前项目: {}",
            video.id
        )));
    }
    Ok(video)
}

pub(super) async fn canvas_media_asset_by_path_for_update<C>(
    db: &C,
    project_id: i64,
    file_path: &str,
) -> Result<Option<media_asset::Model>, CanvasDbError>
where
    C: ConnectionTrait,
{
    let mut query = media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .filter(media_asset::Column::FilePath.eq(file_path.to_string()))
        .order_by_asc(media_asset::Column::Id);
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    Ok(query.one(db).await?)
}
