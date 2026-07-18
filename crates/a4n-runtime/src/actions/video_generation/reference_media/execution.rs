use super::super::{
    AppState, GenerateReferenceMediaVideoInput, GeneratedVideoSummary,
    VIDEO_GENERATION_PROGRESS_TOTAL, VideoGenerationProgress, VideoProviderRequestPurpose,
    ensure_created_video_capacity, ensure_video_capacity, mark_video_generation_failed,
    video_generation_task_scope, video_sidecar_root_project_id,
};
use super::request::build_reference_media_provider_request;

mod provider;
mod record;

pub(super) async fn generate_reference_media_video_inner(
    state: &AppState,
    progress: &mut VideoGenerationProgress<'_>,
    input: GenerateReferenceMediaVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    let GenerateReferenceMediaVideoInput {
        user_id,
        project_id,
        chapter_id,
        input,
    } = input;
    progress.progress(1, VIDEO_GENERATION_PROGRESS_TOTAL, "正在校验参考媒体请求")?;
    progress.check_cancelled("参考媒体视频生成已停止")?;
    video_generation_task_scope(state, user_id, project_id).await?;
    let root_project_id = video_sidecar_root_project_id(state, project_id).await?;
    let context = build_reference_media_provider_request(
        state,
        project_id,
        chapter_id,
        input,
        VideoProviderRequestPurpose::Generate,
    )
    .await?;
    ensure_video_capacity(state, context.cfg.id, context.cfg.max_concurrent).await?;

    progress.progress(2, VIDEO_GENERATION_PROGRESS_TOTAL, "正在创建视频生成记录")?;
    progress.check_cancelled("参考媒体视频生成已停止")?;
    let video = record::create_reference_media_video_generation_record(
        state,
        project_id,
        chapter_id,
        root_project_id,
        &context,
    )
    .await?;
    if let Err(msg) =
        ensure_created_video_capacity(state, context.cfg.id, context.cfg.max_concurrent, video.id)
            .await
    {
        return mark_video_generation_failed(state, video.id, None, &msg).await;
    }

    provider::run_reference_media_video_provider(
        state,
        progress,
        project_id,
        root_project_id,
        video,
        &context,
    )
    .await
}
