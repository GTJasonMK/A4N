use super::ProjectCleanupDbError;
use crate::project::validate_positive_i64;
use a4n_core::models::{
    ai::{generated_image, generated_video},
    character::character_portrait,
    media::media_asset,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use std::collections::BTreeSet;

pub(crate) async fn project_generated_media_paths<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<String>, ProjectCleanupDbError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectCleanupDbError::InvalidArgument)?;
    let mut paths = BTreeSet::new();
    for image in generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .all(db)
        .await?
    {
        paths.insert(image.file_path);
    }
    for video in generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .all(db)
        .await?
    {
        if let Some(path) = video.file_path {
            paths.insert(path);
        }
        if let Some(path) = video.thumbnail_path {
            paths.insert(path);
        }
    }
    for portrait in character_portrait::Entity::find()
        .filter(character_portrait::Column::ProjectId.eq(project_id))
        .all(db)
        .await?
    {
        paths.insert(portrait.image_path);
    }
    for asset in media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .all(db)
        .await?
    {
        if let Some(path) = asset.file_path {
            paths.insert(path);
        }
        if let Some(path) = asset.thumbnail_path {
            paths.insert(path);
        }
    }
    Ok(paths.into_iter().collect())
}
