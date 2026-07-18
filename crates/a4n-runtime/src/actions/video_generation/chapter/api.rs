use super::super::{
    AppState, GenerateChapterVideoInput, GeneratedVideoSummary, VideoGenerationProgress,
    VideoGenerationProgressCallback,
};
use super::execution::generate_chapter_video_inner;
use tokio_util::sync::CancellationToken;

pub async fn generate_chapter_video(
    state: &AppState,
    input: GenerateChapterVideoInput,
    progress: Option<VideoGenerationProgressCallback<'_>>,
    cancel_token: CancellationToken,
) -> Result<GeneratedVideoSummary, String> {
    let mut progress = VideoGenerationProgress::new(progress, cancel_token);
    generate_chapter_video_inner(state, &mut progress, input).await
}
