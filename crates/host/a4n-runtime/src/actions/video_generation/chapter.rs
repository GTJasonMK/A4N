use super::provider_host::RuntimeVideoProviderHostPort;
use super::{
    AppState, GenerateChapterVideoInput, ModelRequestPreview,
    PreviewChapterVideoPromptOverrideInput, model_request_preview, provider_request_step,
};

pub async fn preview_chapter_video_request(
    state: &AppState,
    input: GenerateChapterVideoInput,
    prompt_override: Option<PreviewChapterVideoPromptOverrideInput>,
) -> Result<ModelRequestPreview, String> {
    let project_id = input.project_id;
    let host_port = RuntimeVideoProviderHostPort::new(state);
    let preview = state
        .chapter_video_request_preview_application()
        .preview_chapter(&host_port, input, prompt_override)
        .await
        .map_err(crate::actions::app_error_message)?;
    let mut step = provider_request_step(
        "chapter_video",
        preview.request_title,
        preview.request_kind,
        Some("video_generation_request".to_string()),
        "ready",
        Some(preview.note),
        &preview.request,
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
        "chapter_video_generation",
        "章节视频模型请求",
        vec![step],
    ))
}
