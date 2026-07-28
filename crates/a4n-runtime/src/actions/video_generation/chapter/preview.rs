use super::super::provider_host::RuntimeVideoProviderHostPort;
use super::super::{
    AppState, GenerateChapterVideoInput, ModelRequestPreview,
    PreviewChapterVideoPromptOverrideInput, model_request_preview, provider_request_step,
};

pub async fn preview_chapter_video_request(
    state: &AppState,
    input: GenerateChapterVideoInput,
    prompt_override: Option<PreviewChapterVideoPromptOverrideInput>,
) -> Result<ModelRequestPreview, String> {
    let host_port = RuntimeVideoProviderHostPort::new(state);
    let preview = state
        .video()
        .preview_chapter_video_request_validated(&host_port, input, prompt_override)
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok(model_request_preview(
        "chapter_video_generation",
        "章节视频模型请求",
        vec![provider_request_step(
            "chapter_video",
            preview.request_title,
            preview.request_kind,
            Some("chapter_video_prompt".to_string()),
            "ready",
            Some(preview.note),
            &preview.request,
        )?],
    ))
}
