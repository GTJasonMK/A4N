use super::classification::{
    is_freeform_generated_media_asset_origin, is_freeform_imported_canvas_image_projection,
    is_freeform_imported_canvas_video_projection,
};
use super::contracts::{FreeformGeneratedResetDbError, FreeformGeneratedResetTargets};
use crate::project::validate_positive_i64;
use a4n_core::models::{
    ai::{generated_image, generated_video},
    generation::{generation_spec, prompt_artifact},
    media::media_asset,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect,
};
use std::collections::BTreeSet;

#[derive(Debug, FromQueryResult)]
struct ImageResetTargetRow {
    id: i64,
    file_path: String,
    image_type: String,
    model: String,
    metadata_json: Option<serde_json::Value>,
}

#[derive(Debug, FromQueryResult)]
struct VideoResetTargetRow {
    id: i64,
    file_path: Option<String>,
    thumbnail_path: Option<String>,
    video_type: String,
    workflow_mode: Option<String>,
    model: String,
    metadata_json: Option<serde_json::Value>,
}

#[derive(Debug, FromQueryResult)]
struct MediaAssetResetTargetRow {
    id: i64,
    file_path: Option<String>,
    thumbnail_path: Option<String>,
    origin: String,
}

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
        .select_only()
        .column(generated_image::Column::Id)
        .column(generated_image::Column::FilePath)
        .column(generated_image::Column::ImageType)
        .column(generated_image::Column::Model)
        .column(generated_image::Column::MetadataJson)
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .order_by_asc(generated_image::Column::Id);
    if lock_rows && db.get_database_backend() != DbBackend::Sqlite {
        image_query = image_query.lock_exclusive();
    }
    for image in image_query
        .into_model::<ImageResetTargetRow>()
        .all(db)
        .await?
    {
        if is_freeform_imported_canvas_image_projection(
            &image.image_type,
            &image.model,
            image.metadata_json.as_ref(),
        ) {
            continue;
        }
        paths.insert(image.file_path.clone());
        targets.image_ids.push(image.id);
    }

    let mut video_query = generated_video::Entity::find()
        .select_only()
        .column(generated_video::Column::Id)
        .column(generated_video::Column::FilePath)
        .column(generated_video::Column::ThumbnailPath)
        .column(generated_video::Column::VideoType)
        .column(generated_video::Column::WorkflowMode)
        .column(generated_video::Column::Model)
        .column(generated_video::Column::MetadataJson)
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .order_by_asc(generated_video::Column::Id);
    if lock_rows && db.get_database_backend() != DbBackend::Sqlite {
        video_query = video_query.lock_exclusive();
    }
    for video in video_query
        .into_model::<VideoResetTargetRow>()
        .all(db)
        .await?
    {
        if is_freeform_imported_canvas_video_projection(
            &video.video_type,
            video.workflow_mode.as_deref(),
            &video.model,
            video.metadata_json.as_ref(),
        ) {
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
        .select_only()
        .column(media_asset::Column::Id)
        .column(media_asset::Column::FilePath)
        .column(media_asset::Column::ThumbnailPath)
        .column(media_asset::Column::Origin)
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .order_by_asc(media_asset::Column::Id);
    if lock_rows && db.get_database_backend() != DbBackend::Sqlite {
        asset_query = asset_query.lock_exclusive();
    }
    for asset in asset_query
        .into_model::<MediaAssetResetTargetRow>()
        .all(db)
        .await?
    {
        if !is_freeform_generated_media_asset_origin(&asset.origin) {
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
        .select_only()
        .column(generation_spec::Column::Id)
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .order_by_asc(generation_spec::Column::Id);
    if lock_rows && db.get_database_backend() != DbBackend::Sqlite {
        spec_query = spec_query.lock_exclusive();
    }
    targets.generation_spec_ids = spec_query.into_tuple::<i64>().all(db).await?;
    if !targets.generation_spec_ids.is_empty() {
        let mut prompt_query = prompt_artifact::Entity::find()
            .select_only()
            .column(prompt_artifact::Column::Id)
            .filter(
                prompt_artifact::Column::GenerationSpecId
                    .is_in(targets.generation_spec_ids.clone()),
            )
            .order_by_asc(prompt_artifact::Column::Id);
        if lock_rows && db.get_database_backend() != DbBackend::Sqlite {
            prompt_query = prompt_query.lock_exclusive();
        }
        targets.prompt_artifact_ids = prompt_query.into_tuple::<i64>().all(db).await?;
    }
    targets.media_paths = paths.into_iter().collect();
    Ok(targets)
}
