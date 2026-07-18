use super::super::ChapterVideoPromptRecordFields;
use crate::video::VideoDbError;
use a4n_core::models::chapter::chapter_video_prompt;
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

pub async fn insert_chapter_video_prompt_record<C>(
    db: &C,
    input: ChapterVideoPromptRecordFields,
    status: String,
    now: NaiveDateTime,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    chapter_video_prompt::ActiveModel {
        project_id: Set(input.project_id),
        chapter_id: Set(input.chapter_id),
        style: Set(input.style),
        prompt_type: Set(input.prompt_type),
        workflow_mode: Set(input.workflow_mode),
        shot_number: Set(input.shot_number),
        title: Set(input.title),
        scene_summary: Set(input.scene_summary),
        visual_prompt: Set(input.visual_prompt),
        motion_prompt: Set(input.motion_prompt),
        negative_prompt: Set(input.negative_prompt),
        duration_secs: Set(input.duration_secs),
        aspect_ratio: Set(input.aspect_ratio),
        reference_image_ids_json: Set(input.reference_image_ids_json),
        status: Set(status),
        generated_video_id: Set(None),
        metadata_json: Set(input.metadata_json),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(VideoDbError::Database)
}

pub async fn replace_chapter_video_prompt_record<C>(
    db: &C,
    existing: chapter_video_prompt::Model,
    input: ChapterVideoPromptRecordFields,
    status: Option<String>,
    clear_generated_video_id: bool,
    now: NaiveDateTime,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let mut active: chapter_video_prompt::ActiveModel = existing.into();
    active.project_id = Set(input.project_id);
    active.chapter_id = Set(input.chapter_id);
    active.style = Set(input.style);
    active.prompt_type = Set(input.prompt_type);
    active.workflow_mode = Set(input.workflow_mode);
    active.shot_number = Set(input.shot_number);
    active.title = Set(input.title);
    active.scene_summary = Set(input.scene_summary);
    active.visual_prompt = Set(input.visual_prompt);
    active.motion_prompt = Set(input.motion_prompt);
    active.negative_prompt = Set(input.negative_prompt);
    active.duration_secs = Set(input.duration_secs);
    active.aspect_ratio = Set(input.aspect_ratio);
    active.reference_image_ids_json = Set(input.reference_image_ids_json);
    active.metadata_json = Set(input.metadata_json);
    if clear_generated_video_id {
        active.generated_video_id = Set(None);
    }
    if let Some(status) = status {
        active.status = Set(status);
    }
    active.updated_at = Set(now);
    active.update(db).await.map_err(VideoDbError::Database)
}
