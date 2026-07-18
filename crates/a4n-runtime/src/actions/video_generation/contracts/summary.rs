use super::GeneratedVideoSummary;
use a4n_core::models::ai::generated_video;

pub(crate) fn video_to_summary(v: &generated_video::Model) -> GeneratedVideoSummary {
    GeneratedVideoSummary {
        id: v.id,
        project_id: v.project_id,
        chapter_id: v.chapter_id,
        video_type: v.video_type.clone(),
        workflow_mode: v.workflow_mode.clone(),
        file_path: v.file_path.clone(),
        thumbnail_path: v.thumbnail_path.clone(),
        prompt: v.prompt.clone(),
        negative_prompt: v.negative_prompt.clone(),
        model: v.model.clone(),
        width: v.width,
        height: v.height,
        duration_secs: v.duration_secs,
        fps: v.fps,
        file_size: v.file_size,
        source_video_id: v.source_video_id,
        source_image_id: v.source_image_id,
        status: v.status.clone(),
        provider_job_id: v.provider_job_id.clone(),
        error_message: v.error_message.clone(),
        metadata_json: v.metadata_json.clone(),
        is_favorite: v.is_favorite,
        created_at: Some(v.created_at.to_string()),
        updated_at: Some(v.updated_at.to_string()),
    }
}
