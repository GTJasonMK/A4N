use super::super::super::{
    AppState, ChapterVideoProviderRequestContext, GeneratedVideoSummary, QueueTaskType,
    TaskPriority, VIDEO_GENERATION_PROGRESS_TOTAL, VideoGenerationProgress,
    VideoProviderResultHandling, handle_video_provider_result, mark_video_generation_failed,
    relay_video_sidecar_progress, run_queued_request_with_cancel,
};
use super::{append_video_request_prompt_length_context, video_request_prompt_length_context};
use a4n_core::models::ai::generated_video;

pub(super) async fn run_chapter_video_provider(
    state: &AppState,
    progress: &mut VideoGenerationProgress<'_>,
    project_id: i64,
    root_project_id: Option<i64>,
    video: generated_video::Model,
    request_context: &ChapterVideoProviderRequestContext,
) -> Result<GeneratedVideoSummary, String> {
    let request = request_context.request.clone();
    let prompt_length_context = video_request_prompt_length_context(&request);
    progress.progress(3, VIDEO_GENERATION_PROGRESS_TOTAL, "正在调用视频模型")?;
    if let Err(message) = progress.check_cancelled("视频生成已停止") {
        return mark_video_generation_failed(
            state,
            video.id,
            Some(request_context.prompt.id),
            &message,
        )
        .await;
    }

    let (sidecar_progress_tx, sidecar_progress_rx) = tokio::sync::mpsc::unbounded_channel();
    let request_future = run_queued_request_with_cancel(
        state,
        QueueTaskType::VideoGeneration,
        TaskPriority::Medium,
        Some(project_id),
        progress.cancel_token(),
        {
            let config = request_context.cfg.clone();
            move || async move {
                crate::sidecar_clients::generate_video(
                    state.runtime_profile,
                    state.capabilities(),
                    &config,
                    request,
                    Some(project_id),
                    root_project_id,
                    None,
                    Some(sidecar_progress_tx),
                )
                .await
            }
        },
    );
    let result = relay_video_sidecar_progress(progress, sidecar_progress_rx, request_future)
        .await
        .map_err(|error| append_video_request_prompt_length_context(error, &prompt_length_context));
    handle_video_provider_result(
        state,
        progress,
        video,
        &request_context.cfg,
        result,
        VideoProviderResultHandling {
            prompt_id: Some(request_context.prompt.id),
            completed_cancel_message: Some("视频生成已停止"),
            async_job_progress_message: "视频模型已提交异步任务，开始等待生成结果",
            missing_result_message: "视频 provider 未返回文件或任务 ID",
        },
    )
    .await
}
