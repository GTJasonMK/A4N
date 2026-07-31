use serde_json::json;

use crate::actions::request_preview::{model_request_preview, provider_request_step};

use super::{
    AppState, GenerateCanvasVideoInput, ModelRequestPreview, RuntimeVideoProviderHostPort,
};

pub async fn preview_canvas_video_request(
    state: &AppState,
    input: GenerateCanvasVideoInput,
) -> Result<ModelRequestPreview, String> {
    let host_port = RuntimeVideoProviderHostPort::new(state);
    let preview = state
        .canvas()
        .preview_canvas_video_request_validated(input, &host_port)
        .await
        .map_err(|error| error.to_string())?;
    let payload = json!({
        "config_id": preview.config_id,
        "provider_backend": preview.provider_backend,
        "model": preview.model,
        "graph_id": preview.graph_id,
        "node_id": preview.node_id,
        "request": preview.request,
    });
    Ok(model_request_preview(
        "canvas_video_generation",
        "画布视频模型请求",
        vec![provider_request_step(
            "canvas_video",
            "画布视频 provider 请求",
            "video_generation_request",
            Some("canvas_video_prompt".to_string()),
            "ready",
            preview.provider_input_note,
            &payload,
        )?],
    ))
}
