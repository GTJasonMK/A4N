use a4n_core::{
    error::AppError,
    models::{
        ai::{generated_image, generated_video},
        chapter::{
            chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE},
            chapter_video_prompt,
        },
        character::character_portrait,
    },
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use super::{
    GeneratedImageGraphRow, GeneratedVideoGraphRow, MangaPromptGraphRow, PortraitGraphRow,
    VideoPromptGraphRow,
};

pub(super) struct FamilyAssetGraphRecords {
    pub(super) portraits: Vec<PortraitGraphRow>,
    pub(super) manga_prompts: Vec<MangaPromptGraphRow>,
    pub(super) video_prompts: Vec<VideoPromptGraphRow>,
    pub(super) generated_images: Vec<GeneratedImageGraphRow>,
    pub(super) generated_videos: Vec<GeneratedVideoGraphRow>,
}

pub(super) async fn load_family_asset_graph_records(
    db: &impl ConnectionTrait,
    family_project_ids: &[i64],
) -> Result<FamilyAssetGraphRecords, AppError> {
    let portraits = character_portrait::Entity::find()
        .select_only()
        .column(character_portrait::Column::ProfileId)
        .filter(character_portrait::Column::ProjectId.is_in(family_project_ids.iter().copied()))
        .order_by_asc(character_portrait::Column::Id)
        .into_model::<PortraitGraphRow>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let manga_prompts = chapter_manga_prompt::Entity::find()
        .select_only()
        .column(chapter_manga_prompt::Column::ChapterId)
        .column(chapter_manga_prompt::Column::GeneratedImageId)
        .filter(chapter_manga_prompt::Column::ProjectId.is_in(family_project_ids.iter().copied()))
        .filter(chapter_manga_prompt::Column::PromptType.eq(MANGA_PAGE_PROMPT_TYPE))
        .order_by_asc(chapter_manga_prompt::Column::ChapterId)
        .order_by_asc(chapter_manga_prompt::Column::PageNumber)
        .order_by_asc(chapter_manga_prompt::Column::Id)
        .into_model::<MangaPromptGraphRow>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let video_prompts = chapter_video_prompt::Entity::find()
        .select_only()
        .column(chapter_video_prompt::Column::PromptType)
        .column(chapter_video_prompt::Column::GeneratedVideoId)
        .filter(chapter_video_prompt::Column::ProjectId.is_in(family_project_ids.iter().copied()))
        .order_by_asc(chapter_video_prompt::Column::Id)
        .into_model::<VideoPromptGraphRow>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let generated_images = generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::ImageType)
        .column(generated_image::Column::MetadataJson)
        .filter(generated_image::Column::ProjectId.is_in(family_project_ids.iter().copied()))
        .order_by_asc(generated_image::Column::Id)
        .into_model::<GeneratedImageGraphRow>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let generated_videos = generated_video::Entity::find()
        .select_only()
        .column(generated_video::Column::Status)
        .filter(generated_video::Column::ProjectId.is_in(family_project_ids.iter().copied()))
        .order_by_asc(generated_video::Column::Id)
        .into_model::<GeneratedVideoGraphRow>()
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
