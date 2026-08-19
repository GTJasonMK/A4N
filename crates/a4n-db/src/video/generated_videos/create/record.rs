use super::super::CreateGeneratedVideoRecordInput;
use a4n_core::models::ai::generated_video;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbErr, Set};

pub(super) async fn insert_generated_video_record<C>(
    db: &C,
    input: CreateGeneratedVideoRecordInput,
) -> Result<generated_video::Model, DbErr>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    generated_video::ActiveModel {
        project_id: Set(input.project_id),
        chapter_id: Set(input.chapter_id),
        video_type: Set(input.video_type),
        workflow_mode: Set(Some(input.workflow_mode)),
        file_path: Set(None),
        thumbnail_path: Set(None),
        prompt: Set(input.prompt),
        negative_prompt: Set(input.negative_prompt),
        model: Set(input.model),
        width: Set(input.width),
        height: Set(input.height),
        duration_secs: Set(input.duration_secs),
        fps: Set(input.fps),
        file_size: Set(None),
        source_video_id: Set(input.source_video_id),
        source_image_id: Set(input.source_image_id),
        status: Set(generated_video::STATUS_RUNNING.to_string()),
        provider_job_id: Set(input.provider_job_id),
        error_message: Set(None),
        metadata_json: Set(input.metadata_json),
        is_favorite: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
}
