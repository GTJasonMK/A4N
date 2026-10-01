use a4n_video_host_contract::{
    ChapterVideoGenerationHostPort, ChapterVideoPreviewHostPort, VideoProviderHostPort,
};
use async_trait::async_trait;

use super::*;

#[async_trait]
impl ChapterVideoPreviewHostPort for RuntimeVideoProviderHostPort<'_> {
    async fn resolve_video_config_for_preview(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError> {
        self.resolve_config(config_id).await
    }

    fn resolve_image_transport_for_preview(
        &self,
        backend: &str,
    ) -> Result<ProviderImageInputTransport, AppError> {
        self.resolve_image_transport_setting(backend)
    }

    async fn resolve_chapter_video_image_inputs_for_preview(
        &self,
        input: ChapterVideoImageInput,
    ) -> Result<ResolvedChapterVideoImageInputs, AppError> {
        self.resolve_video_image_inputs(input).await
    }

    async fn chapter_video_provider_request_body_for_preview(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError> {
        self.provider_request_body(input).await
    }
}

#[async_trait]
impl ChapterVideoGenerationHostPort for RuntimeVideoProviderHostPort<'_> {
    async fn resolve_video_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError> {
        self.resolve_config(config_id).await
    }

    async fn resolve_chapter_video_image_inputs(
        &self,
        input: ChapterVideoImageInput,
    ) -> Result<ResolvedChapterVideoImageInputs, AppError> {
        <Self as ChapterVideoPreviewHostPort>::resolve_chapter_video_image_inputs_for_preview(
            self, input,
        )
        .await
    }

    async fn chapter_video_provider_request_body(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError> {
        <Self as ChapterVideoPreviewHostPort>::chapter_video_provider_request_body_for_preview(
            self, input,
        )
        .await
    }

    async fn submit_chapter_video(
        &self,
        input: VideoProviderRequestInput,
        progress: &mut dyn VideoGenerationProgressPort,
    ) -> Result<VideoGenerationResult, AppError> {
        self.submit_video(input, progress).await
    }

    fn runtime_config_snapshot(&self) -> Option<Value> {
        self.redacted_runtime_config_snapshot()
    }

    fn video_provider_host_port(&self) -> &dyn VideoProviderHostPort {
        self
    }
}
