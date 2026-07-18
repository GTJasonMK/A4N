use super::super::super::{
    AppState, GeneratedVideoSummary, QueueTaskType, TaskPriority, VIDEO_GENERATION_PROGRESS_TOTAL,
    VideoGenerationProgress, append_video_request_prompt_length_context,
    handle_canvas_video_provider_result, mark_video_generation_failed,
    relay_video_sidecar_progress, run_queued_request_with_cancel_and_workflow,
    video_request_prompt_length_context,
};
use super::super::request::CanvasVideoProviderRequestContext;
use a4n_core::models::ai::generated_video;

pub(super) async fn run_canvas_video_provider(
    state: &AppState,
    progress: &mut VideoGenerationProgress<'_>,
    root_project_id: Option<i64>,
    video: generated_video::Model,
    context: &CanvasVideoProviderRequestContext,
) -> Result<GeneratedVideoSummary, String> {
    let request = context.request.clone();
    let prompt_length_context = video_request_prompt_length_context(&request);
    progress.progress(3, VIDEO_GENERATION_PROGRESS_TOTAL, "正在调用视频模型")?;
    if let Err(message) = progress.check_cancelled("画布视频生成已停止") {
        return mark_video_generation_failed(state, video.id, None, &message).await;
    }

    let (sidecar_progress_tx, sidecar_progress_rx) = tokio::sync::mpsc::unbounded_channel();
    let request_future = run_queued_request_with_cancel_and_workflow(
        state,
        QueueTaskType::VideoGeneration,
        TaskPriority::Medium,
        Some(context.project_id),
        context.workflow_run_id,
        progress.cancel_token(),
        {
            let config = context.cfg.clone();
            let project_id = context.project_id;
            let workflow_run_id = context.workflow_run_id;
            move || async move {
                crate::sidecar_clients::generate_video(
                    state.runtime_profile,
                    state.capabilities(),
                    &config,
                    request,
                    Some(project_id),
                    root_project_id,
                    workflow_run_id,
                    Some(sidecar_progress_tx),
                )
                .await
            }
        },
    );
    let result = relay_video_sidecar_progress(progress, sidecar_progress_rx, request_future)
        .await
        .map_err(|error| append_video_request_prompt_length_context(error, &prompt_length_context));
    handle_canvas_video_provider_result(state, progress, video, &context.cfg, result).await
}
