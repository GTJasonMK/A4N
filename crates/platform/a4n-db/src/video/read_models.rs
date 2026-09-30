use a4n_db_models::models::ai::generated_image;
use a4n_domain::media::{
    GeneratedImageRecord, GeneratedVideoRecord, ImagePublicationFileFacts,
    UpdateGeneratedImageRecordInput, UpdateGeneratedVideoRecordInput,
};
use a4n_domain::video_prompt::ChapterVideoPromptRecord;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, TransactionTrait};
use serde_json::Value;

use super::VideoDbError;
use super::media_mapping::{generated_image_record_from_model, generated_video_record_from_model};
use super::prompts::record_mapping::{
    chapter_video_prompt_model_from_record, chapter_video_prompt_record_from_model,
};

fn image_records(images: Vec<generated_image::Model>) -> Vec<GeneratedImageRecord> {
    images
        .into_iter()
        .map(generated_image_record_from_model)
        .collect()
}

pub async fn generated_image_by_id<C: ConnectionTrait>(
    db: &C,
    image_id: i64,
) -> Result<Option<GeneratedImageRecord>, DbErr> {
    super::generated_image_by_id(db, image_id)
        .await
        .map(|image| image.map(generated_image_record_from_model))
}

pub async fn generated_images_for_project<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
) -> Result<Vec<GeneratedImageRecord>, DbErr> {
    super::generated_images_for_project(db, project_id)
        .await
        .map(image_records)
}

pub async fn generated_images_for_projects<C: ConnectionTrait>(
    db: &C,
    project_ids: &[i64],
) -> Result<Vec<GeneratedImageRecord>, VideoDbError> {
    super::generated_images_for_projects(db, project_ids)
        .await
        .map(image_records)
}

pub async fn generated_images_for_chapter<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<GeneratedImageRecord>, DbErr> {
    super::generated_images_for_chapter(db, project_id, chapter_id)
        .await
        .map(image_records)
}

pub async fn portrait_reference_images_by_ids<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    image_ids: &[i64],
) -> Result<Vec<GeneratedImageRecord>, crate::character::CharacterPortraitRecordError> {
    crate::character::portrait_reference_images_by_ids(db, project_id, image_ids)
        .await
        .map(image_records)
}

pub async fn validated_video_reference_image_records<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    reference_image_ids_json: Option<&Value>,
) -> Result<Vec<GeneratedImageRecord>, VideoDbError> {
    super::validated_video_reference_image_records(
        db,
        project_id,
        chapter_id,
        reference_image_ids_json,
    )
    .await
    .map(image_records)
}

pub async fn update_generated_image_record<C: ConnectionTrait>(
    db: &C,
    image_id: i64,
    input: UpdateGeneratedImageRecordInput,
) -> Result<GeneratedImageRecord, VideoDbError> {
    super::update_generated_image_record(db, image_id, input)
        .await
        .map(generated_image_record_from_model)
}

pub async fn publish_image_publication<C: ConnectionTrait + TransactionTrait>(
    db: &C,
    publication_id: &str,
    facts: ImagePublicationFileFacts,
) -> Result<GeneratedImageRecord, VideoDbError> {
    super::publish_image_publication(db, publication_id, facts)
        .await
        .map(generated_image_record_from_model)
}

pub async fn generated_video_by_id<C: ConnectionTrait>(
    db: &C,
    video_id: i64,
) -> Result<Option<GeneratedVideoRecord>, DbErr> {
    super::generated_video_by_id(db, video_id)
        .await
        .map(|video| video.map(generated_video_record_from_model))
}

pub async fn generated_videos_for_chapter<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<GeneratedVideoRecord>, DbErr> {
    super::generated_videos_for_chapter(db, project_id, chapter_id)
        .await
        .map(|videos| {
            videos
                .into_iter()
                .map(generated_video_record_from_model)
                .collect()
        })
}

pub async fn update_generated_video_record<C: ConnectionTrait>(
    db: &C,
    video_id: i64,
    input: UpdateGeneratedVideoRecordInput,
) -> Result<GeneratedVideoRecord, VideoDbError> {
    super::update_generated_video_record(db, video_id, input)
        .await
        .map(generated_video_record_from_model)
}

pub async fn chapter_video_prompt_by_id(
    db: &DatabaseConnection,
    prompt_id: i64,
) -> Result<Option<ChapterVideoPromptRecord>, VideoDbError> {
    super::chapter_video_prompt_by_id(db, prompt_id)
        .await
        .map(|prompt| prompt.map(chapter_video_prompt_record_from_model))
}

pub async fn chapter_video_prompts_for_chapter<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<ChapterVideoPromptRecord>, DbErr> {
    super::chapter_video_prompts_for_chapter(db, project_id, chapter_id)
        .await
        .map(|prompts| {
            prompts
                .into_iter()
                .map(chapter_video_prompt_record_from_model)
                .collect()
        })
}

pub async fn with_video_plan_source_sync<C: ConnectionTrait>(
    db: &C,
    prompt: ChapterVideoPromptRecord,
) -> Result<ChapterVideoPromptRecord, VideoDbError> {
    super::with_video_plan_source_sync(db, chapter_video_prompt_model_from_record(prompt))
        .await
        .map(chapter_video_prompt_record_from_model)
}
