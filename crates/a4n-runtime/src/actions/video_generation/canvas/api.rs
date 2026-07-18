use super::super::{
    AppState, GenerateCanvasVideoInput, GeneratedVideoSummary, ModelRequestPreview,
    VideoGenerationProgress, VideoGenerationProgressCallback, VideoProviderRequestPurpose,
    VideoRequestImageMode,
};
use super::execution::generate_canvas_video_inner;
use super::preview::canvas_video_provider_request_preview;
use super::request::build_canvas_video_provider_request;
use tokio_util::sync::CancellationToken;

pub async fn preview_canvas_video_request(
    state: &AppState,
    input: GenerateCanvasVideoInput,
) -> Result<ModelRequestPreview, String> {
    let context = build_canvas_video_provider_request(
        state,
        input,
        VideoProviderRequestPurpose::Preview,
        VideoRequestImageMode::PreviewPlaceholders,
    )
    .await?;
    canvas_video_provider_request_preview(&context)
}

pub async fn generate_canvas_video(
    state: &AppState,
    input: GenerateCanvasVideoInput,
    progress: Option<VideoGenerationProgressCallback<'_>>,
    cancel_token: CancellationToken,
) -> Result<GeneratedVideoSummary, String> {
    let mut progress = VideoGenerationProgress::new(progress, cancel_token);
    generate_canvas_video_inner(state, input, &mut progress).await
}
