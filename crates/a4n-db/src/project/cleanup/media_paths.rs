use super::ProjectCleanupDbError;
use crate::project::validate_positive_i64;
use a4n_core::models::{
    ai::{generated_image, generated_video},
    character::character_portrait,
    media::media_asset,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};
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
    for file_path in generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::FilePath)
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .into_tuple::<String>()
        .all(db)
        .await?
    {
        paths.insert(file_path);
    }
    for (file_path, thumbnail_path) in generated_video::Entity::find()
        .select_only()
        .column(generated_video::Column::FilePath)
        .column(generated_video::Column::ThumbnailPath)
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .into_tuple::<(Option<String>, Option<String>)>()
        .all(db)
        .await?
    {
        if let Some(path) = file_path {
            paths.insert(path);
        }
        if let Some(path) = thumbnail_path {
            paths.insert(path);
        }
    }
    for image_path in character_portrait::Entity::find()
        .select_only()
        .column(character_portrait::Column::ImagePath)
        .filter(character_portrait::Column::ProjectId.eq(project_id))
        .into_tuple::<String>()
        .all(db)
        .await?
    {
        paths.insert(image_path);
    }
    for (file_path, thumbnail_path) in media_asset::Entity::find()
        .select_only()
        .column(media_asset::Column::FilePath)
        .column(media_asset::Column::ThumbnailPath)
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .into_tuple::<(Option<String>, Option<String>)>()
        .all(db)
        .await?
    {
        if let Some(path) = file_path {
            paths.insert(path);
        }
        if let Some(path) = thumbnail_path {
            paths.insert(path);
        }
    }
    Ok(paths.into_iter().collect())
}
