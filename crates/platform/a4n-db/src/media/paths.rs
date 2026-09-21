use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    character::character_portrait,
    media::{image_publication, media_asset},
};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect,
};
use std::collections::BTreeSet;
use std::collections::HashSet;

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
    let referenced_batches = futures::future::join_all(
        candidates
            .chunks(MEDIA_PATH_QUERY_BATCH_SIZE)
            .map(|batch| referenced_media_paths_for_batch(db, batch)),
    )
    .await;
    for referenced in referenced_batches {
        for path in referenced? {
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

pub async fn project_has_active_image_publication<C>(db: &C, project_id: i64) -> Result<bool, DbErr>
where
    C: ConnectionTrait,
{
    use a4n_domain::media::{IMAGE_PUBLICATION_STATUS_PENDING, IMAGE_PUBLICATION_STATUS_PUBLISHED};

    Ok(image_publication::Entity::find()
        .select_only()
        .column(image_publication::Column::PublicationId)
        .filter(image_publication::Column::ProjectId.eq(project_id))
        .filter(image_publication::Column::Status.is_in([
            IMAGE_PUBLICATION_STATUS_PENDING,
            IMAGE_PUBLICATION_STATUS_PUBLISHED,
        ]))
        .into_tuple::<String>()
        .one(db)
        .await?
        .is_some())
}

pub async fn protected_image_publication_cleanup_paths<C>(
    db: &C,
    project_id: i64,
    paths: &[String],
) -> Result<HashSet<String>, DbErr>
where
    C: ConnectionTrait,
{
    use a4n_domain::media::{IMAGE_PUBLICATION_STATUS_PENDING, IMAGE_PUBLICATION_STATUS_PUBLISHED};

    if paths.is_empty() {
        return Ok(HashSet::new());
    }
    let publications = image_publication::Entity::find()
        .filter(image_publication::Column::ProjectId.eq(project_id))
        .filter(image_publication::Column::Status.is_in([
            IMAGE_PUBLICATION_STATUS_PENDING,
            IMAGE_PUBLICATION_STATUS_PUBLISHED,
        ]))
        .filter(
            Condition::any()
                .add(image_publication::Column::StagingPath.is_in(paths.to_vec()))
                .add(image_publication::Column::FinalPath.is_in(paths.to_vec())),
        )
        .all(db)
        .await?;
    let mut protected = HashSet::new();
    for publication in publications {
        if publication.status == IMAGE_PUBLICATION_STATUS_PENDING {
            if paths.contains(&publication.staging_path) {
                protected.insert(publication.staging_path);
            }
            if paths.contains(&publication.final_path) {
                protected.insert(publication.final_path);
            }
        } else if paths.contains(&publication.final_path) {
            protected.insert(publication.final_path);
        }
    }
    Ok(protected)
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
