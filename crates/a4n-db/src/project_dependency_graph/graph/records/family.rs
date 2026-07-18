use a4n_core::{
    error::AppError,
    models::{
        ai::{generated_image, generated_video},
        chapter::{chapter_manga_prompt, chapter_video_prompt},
        character::character_portrait,
    },
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

pub(super) struct FamilyAssetGraphRecords {
    pub(super) portraits: Vec<character_portrait::Model>,
    pub(super) manga_prompts: Vec<chapter_manga_prompt::Model>,
    pub(super) video_prompts: Vec<chapter_video_prompt::Model>,
    pub(super) generated_images: Vec<generated_image::Model>,
    pub(super) generated_videos: Vec<generated_video::Model>,
}

pub(super) async fn load_family_asset_graph_records(
    db: &impl ConnectionTrait,
    family_project_ids: &[i64],
) -> Result<FamilyAssetGraphRecords, AppError> {
    let portraits = character_portrait::Entity::find()
        .filter(character_portrait::Column::ProjectId.is_in(family_project_ids.iter().copied()))
        .order_by_asc(character_portrait::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let manga_prompts = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.is_in(family_project_ids.iter().copied()))
        .order_by_asc(chapter_manga_prompt::Column::ChapterId)
        .order_by_asc(chapter_manga_prompt::Column::PageNumber)
        .order_by_asc(chapter_manga_prompt::Column::PanelNumber)
        .order_by_asc(chapter_manga_prompt::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let video_prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.is_in(family_project_ids.iter().copied()))
        .order_by_asc(chapter_video_prompt::Column::ChapterId)
        .order_by_asc(chapter_video_prompt::Column::WorkflowMode)
        .order_by_asc(chapter_video_prompt::Column::PromptType)
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let generated_images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.is_in(family_project_ids.iter().copied()))
        .order_by_asc(generated_image::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let generated_videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.is_in(family_project_ids.iter().copied()))
        .order_by_asc(generated_video::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;

    Ok(FamilyAssetGraphRecords {
        portraits,
        manga_prompts,
        video_prompts,
        generated_images,
        generated_videos,
    })
}
