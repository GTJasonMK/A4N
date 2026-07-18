use super::super::{
    AppState, GenerateCanvasVideoInput, GeneratedVideoSummary, VIDEO_GENERATION_PROGRESS_TOTAL,
    VideoGenerationProgress, VideoProviderRequestPurpose, VideoRequestImageMode,
    ensure_created_video_capacity, ensure_video_capacity, mark_video_generation_failed,
    video_sidecar_root_project_id,
};
use super::request::build_canvas_video_provider_request;

mod provider;
mod record;

pub(super) async fn generate_canvas_video_inner(
    state: &AppState,
    input: GenerateCanvasVideoInput,
    progress: &mut VideoGenerationProgress<'_>,
) -> Result<GeneratedVideoSummary, String> {
    progress.progress(1, VIDEO_GENERATION_PROGRESS_TOTAL, "正在校验画布视频节点")?;
    progress.check_cancelled("画布视频生成已停止")?;
    let context = build_canvas_video_provider_request(
        state,
        input,
        VideoProviderRequestPurpose::Generate,
        VideoRequestImageMode::ProviderImageInputs,
    )
    .await?;
    let root_project_id = video_sidecar_root_project_id(state, context.project_id).await?;
    ensure_video_capacity(state, context.cfg.id, context.cfg.max_concurrent).await?;

    progress.progress(2, VIDEO_GENERATION_PROGRESS_TOTAL, "正在创建画布视频记录")?;
    progress.check_cancelled("画布视频生成已停止")?;
    let video = record::create_canvas_video_generation_record(state, &context).await?;
    if let Err(msg) =
        ensure_created_video_capacity(state, context.cfg.id, context.cfg.max_concurrent, video.id)
            .await
    {
        return mark_video_generation_failed(state, video.id, None, &msg).await;
    }

    provider::run_canvas_video_provider(state, progress, root_project_id, video, &context).await
}
