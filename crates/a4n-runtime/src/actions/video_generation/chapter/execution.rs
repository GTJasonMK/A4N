use super::super::{
    AppState, GenerateChapterVideoInput, GeneratedVideoSummary, VIDEO_GENERATION_PROGRESS_TOTAL,
    VideoGenerationProgress, VideoGenerationRequest, VideoProviderRequestPurpose,
    VideoRequestImageMode, build_chapter_video_provider_request, ensure_created_video_capacity,
    ensure_video_capacity, mark_video_generation_failed, video_sidecar_root_project_id,
};
use super::scope::video_generation_task_scope;

mod provider;
mod record;

pub(super) async fn generate_chapter_video_inner(
    state: &AppState,
    progress: &mut VideoGenerationProgress<'_>,
    input: GenerateChapterVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    let GenerateChapterVideoInput {
        user_id,
        project_id,
        chapter_id,
        prompt_id,
        config_id,
    } = input;
    progress.progress(
        0,
        VIDEO_GENERATION_PROGRESS_TOTAL,
        "正在检查视频提示词和配置",
    )?;
    progress.check_cancelled("视频生成已停止")?;
    video_generation_task_scope(state, user_id, project_id).await?;
    let request_context = build_chapter_video_provider_request(
        state,
        project_id,
        chapter_id,
        prompt_id,
        user_id,
        config_id,
        VideoProviderRequestPurpose::Generate,
        VideoRequestImageMode::ProviderImageInputs,
        None,
    )
    .await?;
    let root_project_id = video_sidecar_root_project_id(state, project_id).await?;
    ensure_video_capacity(
        state,
        request_context.cfg.id,
        request_context.cfg.max_concurrent,
    )
    .await?;

    progress.progress(2, VIDEO_GENERATION_PROGRESS_TOTAL, "正在创建视频生成记录")?;
    progress.check_cancelled("视频生成已停止")?;
    let video = record::create_chapter_video_generation_record(
        state,
        project_id,
        chapter_id,
        root_project_id,
        &request_context,
    )
    .await?;
    if let Err(msg) = ensure_created_video_capacity(
        state,
        request_context.cfg.id,
        request_context.cfg.max_concurrent,
        video.id,
    )
    .await
    {
        return mark_video_generation_failed(
            state,
            video.id,
            Some(request_context.prompt.id),
            &msg,
        )
        .await;
    }

    provider::run_chapter_video_provider(
        state,
        progress,
        project_id,
        root_project_id,
        video,
        &request_context,
    )
    .await
}

pub(in crate::actions::video_generation) fn video_request_prompt_length_context(
    request: &VideoGenerationRequest,
) -> String {
    format!(
        "实际发送 prompt：{} 字符 / {} UTF-8 字节",
        request.prompt.chars().count(),
        request.prompt.len()
    )
}

pub(in crate::actions::video_generation) fn append_video_request_prompt_length_context(
    err: String,
    context: &str,
) -> String {
    if err.contains(context) {
        err
    } else {
        format!("{err}；{context}")
    }
}
