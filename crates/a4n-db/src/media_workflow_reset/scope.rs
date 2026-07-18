use a4n_core::models::{
    ai::{generated_image, generated_video},
    generation::{generation_spec, prompt_artifact},
    media::media_asset,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};

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
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .order_by_asc(generated_image::Column::Id)
        .all(db)
        .await?;
    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .order_by_asc(generated_video::Column::Id)
        .all(db)
        .await?;
    let generated_assets = media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .filter(media_asset::Column::Origin.ne("imported"))
        .order_by_asc(media_asset::Column::Id)
        .all(db)
        .await?;
    let generation_spec_ids = generation_spec::Entity::find()
        .select_only()
        .column(generation_spec::Column::Id)
        .filter(generation_spec::Column::ProjectId.eq(project_id))
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
        .map(|row| row.file_path.clone())
        .chain(
            videos
                .iter()
                .flat_map(|row| [row.file_path.clone(), row.thumbnail_path.clone()])
                .flatten(),
        )
        .chain(
            generated_assets
                .iter()
                .flat_map(|row| [row.file_path.clone(), row.thumbnail_path.clone()])
                .flatten(),
        )
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
