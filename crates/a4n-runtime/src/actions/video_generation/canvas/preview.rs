use super::super::{
    ModelRequestPreview, Value, model_request_preview, provider_request_snapshot_value,
    provider_request_step,
};
use super::request::CanvasVideoProviderRequestContext;
use serde_json::json;

pub(super) fn canvas_video_provider_request_preview(
    context: &CanvasVideoProviderRequestContext,
) -> Result<ModelRequestPreview, String> {
    let request = canvas_video_provider_preview_payload(context)?;
    Ok(model_request_preview(
        "canvas_video_generation",
        "画布视频模型请求",
        vec![provider_request_step(
            "canvas_video",
            "画布视频 provider 请求",
            "video_generation_request",
            Some("canvas_video_prompt".to_string()),
            "ready",
            context.provider_input_note.clone(),
            &request,
        )?],
    ))
}

pub(super) fn canvas_video_provider_request_snapshot(
    context: &CanvasVideoProviderRequestContext,
) -> Result<Value, String> {
    Ok(provider_request_snapshot_value(
        "canvas_video",
        "画布视频 provider 请求",
        "video_generation_request",
        Some("canvas_video_prompt"),
        context.provider_input_note.clone(),
        canvas_video_provider_preview_payload(context)?,
    ))
}

fn canvas_video_provider_preview_payload(
    context: &CanvasVideoProviderRequestContext,
) -> Result<Value, String> {
    serde_json::to_value(json!({
        "config_id": context.cfg.id,
        "provider_backend": context.cfg.backend,
        "model": context.cfg.model,
        "graph_id": context.graph_id,
        "node_id": context.node_id,
        "request": context.request,
    }))
    .map_err(|err| err.to_string())
}
