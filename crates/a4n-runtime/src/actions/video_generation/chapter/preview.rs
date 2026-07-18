use super::super::{
    AppState, GenerateChapterVideoInput, ModelRequestPreview,
    PreviewChapterVideoPromptOverrideInput, VideoProviderRequestPurpose, VideoRequestImageMode,
    build_chapter_video_provider_request, model_request_preview, provider_request_step,
    video_provider_preview_payload,
};
use super::scope::chapter_video_generation_task_scope;

pub async fn preview_chapter_video_request(
    state: &AppState,
    input: GenerateChapterVideoInput,
    prompt_override: Option<PreviewChapterVideoPromptOverrideInput>,
) -> Result<ModelRequestPreview, String> {
    chapter_video_generation_task_scope(state, &input).await?;
    let project_id = input.project_id;
    let context = build_chapter_video_provider_request(
        state,
        input.project_id,
        input.chapter_id,
        input.prompt_id,
        input.user_id,
        input.config_id,
        VideoProviderRequestPurpose::Preview,
        VideoRequestImageMode::PreviewPlaceholders,
        prompt_override,
    )
    .await?;
    if state.runtime_profile.uses_sidecars() {
        let root_project_id = crate::actions::sidecar_root_project_id(state, project_id).await?;
        let provider_request = crate::sidecar_clients::preview_video_provider_request(
            state.capabilities(),
            &context.cfg,
            context.request,
            Some(project_id),
            root_project_id,
        )
        .await?;
        let note = match context.provider_input_note {
            Some(note) => format!("由 video sidecar 生成真实 provider 请求体；{note}"),
            None => "由 video sidecar 生成真实 provider 请求体".to_string(),
        };
        return Ok(model_request_preview(
            "chapter_video_generation",
            "章节视频模型请求",
            vec![provider_request_step(
                "chapter_video",
                "章节视频 provider 请求",
                "video_provider_request",
                Some("chapter_video_prompt".to_string()),
                "ready",
                Some(note),
                &provider_request,
            )?],
        ));
    }
    let (request_kind, request_title, request_note, request) =
        video_provider_preview_payload(&context)?;
    Ok(model_request_preview(
        "chapter_video_generation",
        "章节视频模型请求",
        vec![provider_request_step(
            "chapter_video",
            request_title,
            request_kind,
            Some("chapter_video_prompt".to_string()),
            "ready",
            Some(request_note),
            &request,
        )?],
    ))
}
