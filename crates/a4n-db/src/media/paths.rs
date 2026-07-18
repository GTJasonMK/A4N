use a4n_core::models::{
    ai::{generated_image, generated_video},
    character::character_portrait,
    media::media_asset,
};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect,
};
use std::collections::BTreeSet;

const MEDIA_PATH_QUERY_BATCH_SIZE: usize = 500;

pub async fn media_path_has_persisted_reference<C>(db: &C, path: &str) -> Result<bool, DbErr>
where
    C: ConnectionTrait,
{
    if media_path_is_blank(path) {
        return Ok(true);
    }
    Ok(filter_unreferenced_media_paths(db, &[path.to_string()])
        .await?
        .is_empty())
}

pub async fn filter_unreferenced_media_paths<C>(
    db: &C,
    paths: &[String],
) -> Result<Vec<String>, DbErr>
where
    C: ConnectionTrait,
{
    let mut unreferenced = normalized_media_paths(paths);
    if unreferenced.is_empty() {
        return Ok(Vec::new());
    }
    let candidates = unreferenced.iter().cloned().collect::<Vec<_>>();
    for batch in candidates.chunks(MEDIA_PATH_QUERY_BATCH_SIZE) {
        for path in referenced_media_paths_for_batch(db, batch).await? {
            unreferenced.remove(&path);
        }
    }

    Ok(unreferenced.into_iter().collect())
}

pub async fn media_path_prefix_has_persisted_reference<C>(
    db: &C,
    prefix: &str,
) -> Result<bool, DbErr>
where
    C: ConnectionTrait,
{
    if media_path_is_blank(prefix) {
        return Ok(true);
    }
    if generated_image::Entity::find()
        .filter(generated_image::Column::FilePath.starts_with(prefix))
        .one(db)
        .await?
        .is_some()
        || generated_video::Entity::find()
            .filter(
                Condition::any()
                    .add(generated_video::Column::FilePath.starts_with(prefix))
                    .add(generated_video::Column::ThumbnailPath.starts_with(prefix)),
            )
            .one(db)
            .await?
            .is_some()
        || character_portrait::Entity::find()
            .filter(character_portrait::Column::ImagePath.starts_with(prefix))
            .one(db)
            .await?
            .is_some()
        || media_asset::Entity::find()
            .filter(
                Condition::any()
                    .add(media_asset::Column::FilePath.starts_with(prefix))
                    .add(media_asset::Column::ThumbnailPath.starts_with(prefix)),
            )
            .one(db)
            .await?
            .is_some()
    {
        return Ok(true);
    }
    Ok(false)
}

async fn referenced_media_paths_for_batch<C>(
    db: &C,
    paths: &[String],
) -> Result<BTreeSet<String>, DbErr>
where
    C: ConnectionTrait,
{
    let mut referenced = BTreeSet::new();
    for path in generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::FilePath)
        .filter(generated_image::Column::FilePath.is_in(paths.iter().cloned()))
        .into_tuple::<String>()
        .all(db)
        .await?
    {
        referenced.insert(path);
    }
    for video in generated_video::Entity::find()
        .filter(
            Condition::any()
                .add(generated_video::Column::FilePath.is_in(paths.iter().cloned()))
                .add(generated_video::Column::ThumbnailPath.is_in(paths.iter().cloned())),
        )
        .all(db)
        .await?
    {
        if let Some(path) = video.file_path {
            referenced.insert(path);
        }
        if let Some(path) = video.thumbnail_path {
            referenced.insert(path);
        }
    }
    for path in character_portrait::Entity::find()
        .select_only()
        .column(character_portrait::Column::ImagePath)
        .filter(character_portrait::Column::ImagePath.is_in(paths.iter().cloned()))
        .into_tuple::<String>()
        .all(db)
        .await?
    {
        referenced.insert(path);
    }
    for asset in media_asset::Entity::find()
        .filter(
            Condition::any()
                .add(media_asset::Column::FilePath.is_in(paths.iter().cloned()))
                .add(media_asset::Column::ThumbnailPath.is_in(paths.iter().cloned())),
        )
        .all(db)
        .await?
    {
        if let Some(path) = asset.file_path {
            referenced.insert(path);
        }
        if let Some(path) = asset.thumbnail_path {
            referenced.insert(path);
        }
    }
    Ok(referenced)
}

pub(super) fn normalized_media_paths(paths: &[String]) -> BTreeSet<String> {
    paths
        .iter()
        .filter(|path| !media_path_is_blank(path))
        .cloned()
        .collect()
}

pub(super) fn media_path_is_blank(path: &str) -> bool {
    path.trim().is_empty()
}
