use super::super::super::{
    AppState, GenerateCanvasVideoInput, VideoProviderRequestPurpose, VideoRequestImageMode, access,
    append_provider_input_note, reference_image_preview_placeholders,
    reference_image_provider_inputs, resolve_video_config, validate_video_duration_for_config_secs,
};
use super::super::binding::{
    canvas_video_provider_image_binding, default_canvas_video_image_transport,
};
use super::context::CanvasVideoProviderRequestContext;
use super::inputs::resolve_canvas_image_input;
use a4n_video::VideoGenerationRequest;
use serde_json::json;

pub(in crate::actions::video_generation::canvas) async fn build_canvas_video_provider_request(
    state: &AppState,
    input: GenerateCanvasVideoInput,
    purpose: VideoProviderRequestPurpose,
    image_mode: VideoRequestImageMode,
) -> Result<CanvasVideoProviderRequestContext, String> {
    access::verify_project_editor_string(state, input.user_id, input.project_id).await?;
    let project = state
        .orchestrator
        .project_by_id(input.project_id)
        .await
        .map_err(crate::actions::app_error_message)?
        .ok_or_else(|| format!("项目不存在: {}", input.project_id))?;
    if project.project_kind != "video" || project.source_mode != "freeform" {
        return Err("GenerateVideoNode 当前只支持自由视频项目".to_string());
    }
    let cfg = resolve_video_config(state, input.config_id).await?;
    let duration_secs = input.duration_secs.unwrap_or(5);
    if purpose == VideoProviderRequestPurpose::Generate {
        validate_video_duration_for_config_secs(&cfg, duration_secs)?;
    }
    let width = input.width.unwrap_or(cfg.default_width);
    let height = input.height.unwrap_or(cfg.default_height);
    if width <= 0 || height <= 0 {
        return Err("画布视频宽高必须为正整数".to_string());
    }
    let fps = input.fps.unwrap_or(cfg.default_fps);
    if fps <= 0 {
        return Err("画布视频 fps 必须为正整数".to_string());
    }
    let prompt = input.prompt.trim().to_string();
    if prompt.is_empty() {
        return Err("GenerateVideoNode 缺少有效 prompt 输入".to_string());
    }
    let mut provider_input_note = None;
    let image_transport = default_canvas_video_image_transport()?;
    let image_binding =
        canvas_video_provider_image_binding(&cfg, &input, &mut provider_input_note)?;
    let first_frame_image_url = resolve_canvas_image_input(
        state,
        &input,
        image_binding.first_frame_image_id,
        image_mode,
        image_transport,
        &mut provider_input_note,
    )
    .await?;
    let last_frame_image_url = resolve_canvas_image_input(
        state,
        &input,
        image_binding.last_frame_image_id,
        image_mode,
        image_transport,
        &mut provider_input_note,
    )
    .await?;
    let reference_ids_json = json!(image_binding.reference_image_ids);
    let reference_image_urls = match image_mode {
        VideoRequestImageMode::ProviderImageInputs => {
            reference_image_provider_inputs(
                state,
                input.user_id,
                input.project_id,
                0,
                Some(&reference_ids_json),
                image_transport,
            )
            .await?
        }
        VideoRequestImageMode::PreviewPlaceholders => {
            let placeholders = reference_image_preview_placeholders(
                state,
                input.user_id,
                input.project_id,
                0,
                Some(&reference_ids_json),
            )
            .await?;
            for note in placeholders.notes {
                append_provider_input_note(&mut provider_input_note, note);
            }
            placeholders.urls
        }
    };
    let request = VideoGenerationRequest {
        prompt,
        negative_prompt: input
            .negative_prompt
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        width,
        height,
        duration_secs,
        fps,
        model: cfg.model.clone(),
        first_frame_image_url,
        last_frame_image_url,
        reference_image_urls,
        reference_media: None,
    };
    Ok(CanvasVideoProviderRequestContext {
        project_id: input.project_id,
        graph_id: input.graph_id,
        node_id: input.node_id,
        workflow_run_id: input.workflow_run_id,
        source_node_ids: input.source_node_ids,
        cfg,
        request,
        provider_input_note,
        first_frame_image_id: input.first_frame_image_id,
        last_frame_image_id: input.last_frame_image_id,
        reference_image_ids: image_binding.reference_image_ids,
    })
}
