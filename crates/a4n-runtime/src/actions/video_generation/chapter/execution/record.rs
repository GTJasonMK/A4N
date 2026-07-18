use super::super::super::{
    AppState, ChapterVideoProviderRequestContext, CreateVideoRecord,
    chapter_video_provider_request_snapshot_for_state, started_video_generation_run_input,
};
use a4n_core::models::ai::generated_video;
use serde_json::json;

pub(super) async fn create_chapter_video_generation_record(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    root_project_id: Option<i64>,
    request_context: &ChapterVideoProviderRequestContext,
) -> Result<generated_video::Model, String> {
    let provider_request_snapshot = chapter_video_provider_request_snapshot_for_state(
        state,
        project_id,
        root_project_id,
        request_context,
    )
    .await?;
    let metadata = json!({
        "config_id": request_context.cfg.id,
        "prompt_id": request_context.prompt.id,
        "shot_number": request_context.prompt.shot_number,
        "source": request_context.prompt_source,
        "source_prompt_type": request_context.source_prompt_type,
        "video_execution_identity": request_context.video_execution_identity,
        "generation_asset_fingerprint": request_context.video_execution_identity.get("generation_asset_fingerprint"),
        "prompt_output_language": request_context.video_execution_identity.get("prompt_output_language"),
        "prompt_reference_image_ids": request_context.prompt_reference_image_ids,
        "prompt_reference_image_count": request_context.prompt_reference_image_count,
        "provider_reference_image_ids": request_context.provider_reference_image_ids,
        "provider_reference_image_count": request_context.provider_reference_image_count,
        "start_frame_image_id": request_context.boundary_frame_ids.first,
        "boundary_frame_image_ids": {
            "first": request_context.boundary_frame_ids.first,
            "last": request_context.boundary_frame_ids.last
        },
        "boundary_frame_image_count": usize::from(request_context.boundary_frame_ids.first.is_some())
            + usize::from(request_context.boundary_frame_ids.last.is_some()),
        "boundary_frame_image_url_count": usize::from(request_context.first_frame_image_url_available)
            + usize::from(request_context.last_frame_image_url_available),
        "boundary_frame_request": {
            "first_frame_image_url": request_context.first_frame_image_url_available,
            "last_frame_image_url": request_context.last_frame_image_url_available,
        },
        "provider_request_snapshot": provider_request_snapshot,
        "runtime_config_snapshot": crate::runtime_config::redacted_current_snapshot_value(),
    });
    let video_record = CreateVideoRecord {
        project_id,
        chapter_id: Some(chapter_id),
        video_type: "chapter_video".into(),
        workflow_mode: Some(request_context.workflow_mode.clone()),
        prompt: request_context.request.prompt.clone(),
        negative_prompt: request_context.request.negative_prompt.clone(),
        model: request_context.cfg.model.clone(),
        width: request_context.request.width,
        height: request_context.request.height,
        duration_secs: request_context.request.duration_secs as f32,
        fps: request_context.request.fps,
        source_video_id: None,
        source_image_id: request_context.boundary_frame_ids.first,
        provider_job_id: None,
        metadata_json: Some(metadata),
    };
    let generation_run = started_video_generation_run_input(
        &video_record,
        "generate_chapter_video",
        "章节视频 provider 生成",
        request_context.cfg.backend.clone(),
        Some("chapter_video_prompt"),
        provider_request_snapshot,
    );
    state
        .orchestrator
        .create_prompt_video_record_with_generation_run(
            video_record,
            generation_run,
            request_context.prompt.id,
        )
        .await
        .map_err(crate::actions::app_error_message)
}
