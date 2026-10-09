use serde_json::json;

use crate::actions::request_preview::{model_request_preview, provider_request_step};

use super::{
    AppState, GenerateCanvasVideoInput, ModelRequestPreview, RuntimeVideoProviderHostPort,
};

pub(crate) async fn preview_canvas_video_request(
    state: &AppState,
    input: GenerateCanvasVideoInput,
) -> Result<ModelRequestPreview, String> {
    let project_id = input.project_id;
    let host_port = RuntimeVideoProviderHostPort::new(state);
    let preview = state
        .canvas_video_request_preview_application()
        .preview_canvas(input, &host_port)
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
    let mut step = provider_request_step(
        "canvas_video",
        "画布视频 provider 请求",
        "video_generation_request",
        Some("video_generation_request".to_string()),
        "ready",
        None,
        &payload,
    )?;
    crate::actions::request_preview::attach_prompt_assembly_metadata(
        state,
        Some(project_id),
        "video_generation.provider",
        "video_generation_request",
        &mut step,
    )
    .await?;
    Ok(model_request_preview(
        "canvas_video_generation",
        "画布视频模型请求",
        vec![step],
    ))
}
