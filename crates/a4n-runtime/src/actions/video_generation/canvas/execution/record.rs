use super::super::super::{AppState, CreateVideoRecord, started_video_generation_run_input};
use super::super::{
    preview::canvas_video_provider_request_snapshot, request::CanvasVideoProviderRequestContext,
};
use a4n_core::models::ai::generated_video;
use serde_json::json;

pub(super) async fn create_canvas_video_generation_record(
    state: &AppState,
    context: &CanvasVideoProviderRequestContext,
) -> Result<generated_video::Model, String> {
    let provider_request_snapshot = canvas_video_provider_request_snapshot(context)?;
    let metadata = json!({
        "canvas": {
            "graph_id": context.graph_id,
            "node_id": context.node_id,
            "workflow_run_id": context.workflow_run_id,
            "source_node_ids": context.source_node_ids,
        },
        "config_id": context.cfg.id,
        "first_frame_image_id": context.first_frame_image_id,
        "last_frame_image_id": context.last_frame_image_id,
        "reference_image_ids": context.reference_image_ids,
        "reference_image_count": context.reference_image_ids.len(),
        "provider_request_snapshot": provider_request_snapshot,
        "runtime_config_snapshot": crate::runtime_config::redacted_current_snapshot_value(),
    });
    let video_record = CreateVideoRecord {
        project_id: context.project_id,
        chapter_id: None,
        video_type: "canvas_video".to_string(),
        workflow_mode: Some("canvas_video".to_string()),
        prompt: context.request.prompt.clone(),
        negative_prompt: context.request.negative_prompt.clone(),
        model: context.cfg.model.clone(),
        width: context.request.width,
        height: context.request.height,
        duration_secs: context.request.duration_secs as f32,
        fps: context.request.fps,
        source_video_id: None,
        source_image_id: context.first_frame_image_id,
        provider_job_id: None,
        metadata_json: Some(metadata),
    };
    let generation_run = started_video_generation_run_input(
        &video_record,
        "generate_canvas_video",
        "画布视频 provider 生成",
        context.cfg.backend.clone(),
        Some("canvas_video_prompt"),
        provider_request_snapshot,
    );
    state
        .orchestrator
        .create_video_record_with_generation_run(video_record, generation_run)
        .await
        .map_err(crate::actions::app_error_message)
}
