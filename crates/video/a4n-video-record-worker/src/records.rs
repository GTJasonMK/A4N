use super::*;

pub(super) fn video_summary(
    video: &a4n_db_models::models::ai::generated_video::Model,
) -> GeneratedVideoSummary {
    GeneratedVideoSummary {
        id: video.id,
        project_id: video.project_id,
        chapter_id: video.chapter_id,
        video_type: video.video_type.clone(),
        workflow_mode: video.workflow_mode.clone(),
        file_path: video.file_path.clone(),
        thumbnail_path: video.thumbnail_path.clone(),
        prompt: video.prompt.clone(),
        negative_prompt: video.negative_prompt.clone(),
        model: video.model.clone(),
        width: video.width,
        height: video.height,
        duration_secs: video.duration_secs,
        fps: video.fps,
        file_size: video.file_size,
        source_video_id: video.source_video_id,
        source_image_id: video.source_image_id,
        status: video.status.clone(),
        provider_job_id: video.provider_job_id.clone(),
        error_message: video.error_message.clone(),
        metadata_json: video.metadata_json.clone(),
        timeline_item_id: video.timeline_item_id,
        is_favorite: video.is_favorite,
        created_at: video.created_at.and_utc().to_rfc3339(),
        updated_at: video.updated_at.and_utc().to_rfc3339(),
    }
}

pub(super) fn image_summary(
    image: &a4n_db_models::models::ai::generated_image::Model,
) -> GeneratedImageSummary {
    GeneratedImageSummary {
        id: image.id,
        project_id: image.project_id,
        image_type: image.image_type.clone(),
        file_path: image.file_path.clone(),
        prompt: image.prompt.clone(),
        model: image.model.clone(),
        width: image.width,
        height: image.height,
        file_size: image.file_size,
        ai_score: image.ai_score,
        is_favorite: image.is_favorite,
        is_system_managed: a4n_domain::image::is_system_managed_image_type(&image.image_type),
        chapter_id: image.chapter_id,
        panel_id: image.panel_id.clone(),
        metadata_json: image.metadata_json.clone(),
        created_at: image.created_at.and_utc().to_rfc3339(),
    }
}

pub(super) async fn verify_project_permission(
    database: &DbHandle,
    project_id: i64,
    user_id: i64,
    role: ProjectAccessRole,
) -> Result<(), WorkerFailure> {
    a4n_db::collaboration::verify_project_permission(&database, project_id, user_id, role)
        .await
        .map(|_| ())
        .map_err(collaboration_error)
}
