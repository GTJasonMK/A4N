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
        .select_only()
        .column(generated_image::Column::Id)
        .filter(generated_image::Column::FilePath.starts_with(prefix))
        .into_tuple::<i64>()
        .one(db)
        .await?
        .is_some()
        || generated_video::Entity::find()
            .select_only()
            .column(generated_video::Column::Id)
            .filter(
                Condition::any()
                    .add(generated_video::Column::FilePath.starts_with(prefix))
                    .add(generated_video::Column::ThumbnailPath.starts_with(prefix)),
            )
            .into_tuple::<i64>()
            .one(db)
            .await?
            .is_some()
        || character_portrait::Entity::find()
            .select_only()
            .column(character_portrait::Column::Id)
            .filter(character_portrait::Column::ImagePath.starts_with(prefix))
            .into_tuple::<i64>()
            .one(db)
            .await?
            .is_some()
        || media_asset::Entity::find()
            .select_only()
            .column(media_asset::Column::Id)
            .filter(
                Condition::any()
                    .add(media_asset::Column::FilePath.starts_with(prefix))
                    .add(media_asset::Column::ThumbnailPath.starts_with(prefix)),
            )
            .into_tuple::<i64>()
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
    let (images_result, videos_result, portraits_result, assets_result) = futures::join!(
        generated_image::Entity::find()
            .select_only()
            .column(generated_image::Column::FilePath)
            .filter(generated_image::Column::FilePath.is_in(paths.iter().cloned()))
            .into_tuple::<String>()
            .all(db),
        generated_video::Entity::find()
            .select_only()
            .column(generated_video::Column::FilePath)
            .column(generated_video::Column::ThumbnailPath)
            .filter(
                Condition::any()
                    .add(generated_video::Column::FilePath.is_in(paths.iter().cloned()))
                    .add(generated_video::Column::ThumbnailPath.is_in(paths.iter().cloned())),
            )
            .into_tuple::<(Option<String>, Option<String>)>()
            .all(db),
        character_portrait::Entity::find()
            .select_only()
            .column(character_portrait::Column::ImagePath)
            .filter(character_portrait::Column::ImagePath.is_in(paths.iter().cloned()))
            .into_tuple::<String>()
            .all(db),
        media_asset::Entity::find()
            .select_only()
            .column(media_asset::Column::FilePath)
            .column(media_asset::Column::ThumbnailPath)
            .filter(
                Condition::any()
                    .add(media_asset::Column::FilePath.is_in(paths.iter().cloned()))
                    .add(media_asset::Column::ThumbnailPath.is_in(paths.iter().cloned())),
            )
            .into_tuple::<(Option<String>, Option<String>)>()
            .all(db),
    );
    for path in images_result? {
        referenced.insert(path);
    }
    for video in videos_result? {
        if let Some(path) = video.0 {
            referenced.insert(path);
        }
        if let Some(path) = video.1 {
            referenced.insert(path);
        }
    }
    for path in portraits_result? {
        referenced.insert(path);
    }
    for asset in assets_result? {
        if let Some(path) = asset.0 {
            referenced.insert(path);
        }
        if let Some(path) = asset.1 {
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
