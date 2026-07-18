use super::super::super::{
    AppState, GeneratedVideoSummary, QueueTaskType, TaskPriority, VIDEO_GENERATION_PROGRESS_TOTAL,
    VideoGenerationProgress, VideoProviderResultHandling,
    append_video_request_prompt_length_context, handle_video_provider_result,
    mark_video_generation_failed, relay_video_sidecar_progress, run_queued_request_with_cancel,
    video_request_prompt_length_context,
};
use super::super::contracts::ReferenceMediaProviderRequestContext;
use a4n_core::models::ai::generated_video;

pub(super) async fn run_reference_media_video_provider(
    state: &AppState,
    progress: &mut VideoGenerationProgress<'_>,
    project_id: i64,
    root_project_id: Option<i64>,
    video: generated_video::Model,
    context: &ReferenceMediaProviderRequestContext,
) -> Result<GeneratedVideoSummary, String> {
    progress.progress(3, VIDEO_GENERATION_PROGRESS_TOTAL, "正在调用 Seedance 2.0")?;
    if let Err(message) = progress.check_cancelled("参考媒体视频生成已停止") {
        return mark_video_generation_failed(state, video.id, None, &message).await;
    }
    let request = context.request.clone();
    let prompt_length_context = video_request_prompt_length_context(&request);
    let (sidecar_progress_tx, sidecar_progress_rx) = tokio::sync::mpsc::unbounded_channel();
    let request_future = run_queued_request_with_cancel(
        state,
        QueueTaskType::VideoGeneration,
        TaskPriority::Medium,
        Some(project_id),
        progress.cancel_token(),
        {
            let config = context.cfg.clone();
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
        &context.cfg,
        result,
        VideoProviderResultHandling {
            prompt_id: None,
            completed_cancel_message: None,
            async_job_progress_message: "Seedance 2.0 已提交异步任务，开始等待生成结果",
            missing_result_message: "Seedance 2.0 未返回文件或任务 ID",
        },
    )
    .await
}
