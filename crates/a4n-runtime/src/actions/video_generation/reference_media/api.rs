use super::super::*;
#[cfg(feature = "embedded-media-providers")]
use super::contracts::ReferenceMediaProviderRequestPreviewContext;
use super::execution::generate_reference_media_video_inner;
#[cfg(feature = "embedded-media-providers")]
use super::preview::reference_media_provider_request_preview;
use super::request::build_reference_media_provider_request;

pub async fn preview_video_provider_request_by_video_id(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    video_id: i64,
) -> Result<ModelRequestPreview, String> {
    let video = state
        .orchestrator
        .generated_video_by_id(video_id)
        .await
        .map_err(crate::actions::app_error_message)?
        .ok_or_else(|| "视频记录不存在".to_string())?;
    access::verify_project_editor_string(state, user_id, video.project_id).await?;
    resolve_project_chapter_video_source(
        state,
        project_id,
        chapter_id,
        "预览历史视频 provider 请求",
        false,
    )
    .await?;
    if video.project_id != project_id || video.chapter_id != Some(chapter_id) {
        return Err("视频记录不属于该章节".to_string());
    }
    validate_video_provider_request_snapshot_owner(&video)?;
    model_request_preview_from_provider_snapshot(
        &video,
        "video_provider_request_record",
        "历史视频模型请求",
    )
}

pub async fn preview_reference_media_video_request(
    state: &AppState,
    input: GenerateReferenceMediaVideoInput,
) -> Result<ModelRequestPreview, String> {
    let GenerateReferenceMediaVideoInput {
        user_id,
        project_id,
        chapter_id,
        input,
    } = input;
    video_generation_task_scope(state, user_id, project_id).await?;
    let context = build_reference_media_provider_request(
        state,
        project_id,
        chapter_id,
        input,
        VideoProviderRequestPurpose::Preview,
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
            "reference_media_video_generation",
            "参考媒体视频模型请求",
            vec![provider_request_step(
                "reference_media_video",
                "参考媒体视频 provider 请求",
                "video_provider_request",
                Some("reference_media_video".to_string()),
                "ready",
                Some(note),
                &provider_request,
            )?],
        ));
    }

    #[cfg(feature = "embedded-media-providers")]
    {
        return reference_media_provider_request_preview(
            "reference_media_video_generation",
            "参考媒体视频模型请求",
            "当前表单",
            &ReferenceMediaProviderRequestPreviewContext {
                request: context.request,
                config_id: Some(context.cfg.id),
                provider_backend: Some(context.cfg.backend),
                provider_input_note: context.provider_input_note,
            },
        );
    }

    #[cfg(not(feature = "embedded-media-providers"))]
    Err("参考媒体视频 provider 请求预览不可用：当前 runtime 既未启用 sidecar profile，也未编译 embedded-media-providers".to_string())
}

pub async fn generate_reference_media_video(
    state: &AppState,
    input: GenerateReferenceMediaVideoInput,
    progress: Option<VideoGenerationProgressCallback<'_>>,
    cancel_token: CancellationToken,
) -> Result<GeneratedVideoSummary, String> {
    let mut progress = VideoGenerationProgress::new(progress, cancel_token);
    generate_reference_media_video_inner(state, &mut progress, input).await
}

pub async fn reference_media_video_generation_task_scope(
    state: &AppState,
    input: &GenerateReferenceMediaVideoInput,
) -> Result<VideoGenerationTaskScope, String> {
    video_generation_task_scope(state, input.user_id, input.project_id).await
}
