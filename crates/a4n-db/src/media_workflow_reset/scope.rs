use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    generation::{generation_spec, prompt_artifact},
    media::{media_asset, media_provenance},
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect,
};

#[derive(Debug, FromQueryResult)]
struct ImageScopeRow {
    id: i64,
    file_path: String,
}

#[derive(Debug, FromQueryResult)]
struct VideoScopeRow {
    id: i64,
    file_path: Option<String>,
    thumbnail_path: Option<String>,
}

#[derive(Debug, FromQueryResult)]
struct MediaAssetScopeRow {
    id: i64,
    file_path: Option<String>,
    thumbnail_path: Option<String>,
}

pub(super) struct MediaWorkflowV2ResetScope {
    pub image_ids: Vec<i64>,
    pub video_ids: Vec<i64>,
    pub media_asset_ids: Vec<i64>,
    pub generation_spec_ids: Vec<i64>,
    pub prompt_artifact_ids: Vec<i64>,
    pub media_paths: Vec<String>,
}

pub(super) async fn load_media_workflow_v2_reset_scope<C>(
    db: &C,
    project_id: i64,
) -> Result<MediaWorkflowV2ResetScope, DbErr>
where
    C: ConnectionTrait,
{
    let (protected_image_ids, _) = crate::chapter_visual_plan::visual_plan_resource_ids(db).await?;
    let all_images = generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::Id)
        .column(generated_image::Column::FilePath)
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .order_by_asc(generated_image::Column::Id)
        .into_model::<ImageScopeRow>()
        .all(db)
        .await?;
    let protected_paths = if protected_image_ids.is_empty() {
        std::collections::BTreeSet::new()
    } else {
        generated_image::Entity::find()
            .select_only()
            .column(generated_image::Column::FilePath)
            .filter(generated_image::Column::Id.is_in(protected_image_ids.iter().copied()))
            .into_tuple::<String>()
            .all(db)
            .await?
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
    };
    let images = all_images
        .into_iter()
        .filter(|image| !protected_image_ids.contains(&image.id))
        .collect::<Vec<_>>();
    let videos = generated_video::Entity::find()
        .select_only()
        .column(generated_video::Column::Id)
        .column(generated_video::Column::FilePath)
        .column(generated_video::Column::ThumbnailPath)
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .order_by_asc(generated_video::Column::Id)
        .into_model::<VideoScopeRow>()
        .all(db)
        .await?;
    let all_generated_assets = media_asset::Entity::find()
        .select_only()
        .column(media_asset::Column::Id)
        .column(media_asset::Column::FilePath)
        .column(media_asset::Column::ThumbnailPath)
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .filter(media_asset::Column::Origin.ne("imported"))
        .order_by_asc(media_asset::Column::Id)
        .into_model::<MediaAssetScopeRow>()
        .all(db)
        .await?;
    let protected_asset_ids = all_generated_assets
        .iter()
        .filter(|asset| {
            asset
                .file_path
                .as_ref()
                .is_some_and(|path| protected_paths.contains(path))
        })
        .map(|asset| asset.id)
        .collect::<Vec<_>>();
    let protected_spec_ids = if protected_asset_ids.is_empty() {
        Vec::new()
    } else {
        media_provenance::Entity::find()
            .filter(media_provenance::Column::MediaAssetId.is_in(protected_asset_ids.clone()))
            .all(db)
            .await?
            .into_iter()
            .filter_map(|row| row.generation_spec_id)
            .collect::<Vec<_>>()
    };
    let generated_assets = all_generated_assets
        .into_iter()
        .filter(|asset| !protected_asset_ids.contains(&asset.id))
        .collect::<Vec<_>>();
    let generation_spec_ids = generation_spec::Entity::find()
        .select_only()
        .column(generation_spec::Column::Id)
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(generation_spec::Column::Id.is_not_in(protected_spec_ids))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let prompt_artifact_ids = if generation_spec_ids.is_empty() {
        Vec::new()
    } else {
        prompt_artifact::Entity::find()
            .select_only()
            .column(prompt_artifact::Column::Id)
            .filter(prompt_artifact::Column::GenerationSpecId.is_in(generation_spec_ids.clone()))
            .into_tuple::<i64>()
            .all(db)
            .await?
    };
    let image_ids = images.iter().map(|row| row.id).collect::<Vec<_>>();
    let video_ids = videos.iter().map(|row| row.id).collect::<Vec<_>>();
    let media_asset_ids = generated_assets
        .iter()
        .map(|row| row.id)
        .collect::<Vec<_>>();
    let media_paths = images
        .iter()
        .map(|row| row.file_path.as_str())
        .chain(
            videos
                .iter()
                .flat_map(|row| [row.file_path.as_ref(), row.thumbnail_path.as_ref()])
                .flatten()
                .map(String::as_str),
        )
        .chain(
            generated_assets
                .iter()
                .flat_map(|row| [row.file_path.as_ref(), row.thumbnail_path.as_ref()])
                .flatten()
                .map(String::as_str),
        )
        .map(str::to_owned)
        .filter(|path| !path.trim().is_empty())
        .collect::<Vec<_>>();

    Ok(MediaWorkflowV2ResetScope {
        image_ids,
        video_ids,
        media_asset_ids,
        generation_spec_ids,
        prompt_artifact_ids,
        media_paths,
    })
}
