use super::classification::{
    is_freeform_generated_media_asset, is_freeform_imported_canvas_image,
    is_freeform_imported_canvas_video,
};
use super::contracts::{FreeformGeneratedResetDbError, FreeformGeneratedResetTargets};
use crate::project::validate_positive_i64;
use a4n_core::models::{
    ai::{generated_image, generated_video},
    generation::{generation_spec, prompt_artifact},
    media::media_asset,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};
use std::collections::BTreeSet;

pub(super) async fn freeform_generated_reset_targets_in<C>(
    db: &C,
    project_id: i64,
    lock_rows: bool,
) -> Result<FreeformGeneratedResetTargets, FreeformGeneratedResetDbError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(project_id, "project_id")
        .map_err(FreeformGeneratedResetDbError::InvalidArgument)?;
    let mut targets = FreeformGeneratedResetTargets::default();
    let mut paths = BTreeSet::new();

    let mut image_query = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .order_by_asc(generated_image::Column::Id);
    if lock_rows && db.get_database_backend() != DbBackend::Sqlite {
        image_query = image_query.lock_exclusive();
    }
    for image in image_query.all(db).await? {
        if is_freeform_imported_canvas_image(&image) {
            continue;
        }
        paths.insert(image.file_path.clone());
        targets.image_ids.push(image.id);
    }

    let mut video_query = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .order_by_asc(generated_video::Column::Id);
    if lock_rows && db.get_database_backend() != DbBackend::Sqlite {
        video_query = video_query.lock_exclusive();
    }
    for video in video_query.all(db).await? {
        if is_freeform_imported_canvas_video(&video) {
            continue;
        }
        if let Some(path) = video.file_path.as_ref() {
            paths.insert(path.clone());
        }
        if let Some(path) = video.thumbnail_path.as_ref() {
            paths.insert(path.clone());
        }
        targets.video_ids.push(video.id);
    }

    let mut asset_query = media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .order_by_asc(media_asset::Column::Id);
    if lock_rows && db.get_database_backend() != DbBackend::Sqlite {
        asset_query = asset_query.lock_exclusive();
    }
    for asset in asset_query.all(db).await? {
        if !is_freeform_generated_media_asset(&asset) {
            continue;
        }
        if let Some(path) = asset.file_path.as_ref() {
            paths.insert(path.clone());
        }
        if let Some(path) = asset.thumbnail_path.as_ref() {
            paths.insert(path.clone());
        }
        targets.media_asset_ids.push(asset.id);
    }

    let mut spec_query = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .order_by_asc(generation_spec::Column::Id);
    if lock_rows && db.get_database_backend() != DbBackend::Sqlite {
        spec_query = spec_query.lock_exclusive();
    }
    targets.generation_spec_ids = spec_query
        .all(db)
        .await?
        .into_iter()
        .map(|row| row.id)
        .collect::<Vec<_>>();
    if !targets.generation_spec_ids.is_empty() {
        let mut prompt_query = prompt_artifact::Entity::find()
            .filter(
                prompt_artifact::Column::GenerationSpecId
                    .is_in(targets.generation_spec_ids.clone()),
            )
            .order_by_asc(prompt_artifact::Column::Id);
        if lock_rows && db.get_database_backend() != DbBackend::Sqlite {
            prompt_query = prompt_query.lock_exclusive();
        }
        targets.prompt_artifact_ids = prompt_query
            .all(db)
            .await?
            .into_iter()
            .map(|row| row.id)
            .collect::<Vec<_>>();
    }
    targets.media_paths = paths.into_iter().collect();
    Ok(targets)
}
