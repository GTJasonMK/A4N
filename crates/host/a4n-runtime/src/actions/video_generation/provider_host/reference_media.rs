use a4n_service_protocol::GenerationArtifactRef;
use a4n_video_host_contract::{
    ReferenceMediaVideoGenerationHostPort, ReferenceMediaVideoPreviewHostPort,
    VideoProviderHostPort,
};
use async_trait::async_trait;

use super::*;

#[async_trait]
impl ReferenceMediaVideoPreviewHostPort for RuntimeVideoProviderHostPort<'_> {
    async fn resolve_video_config_for_preview(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError> {
        self.resolve_config(config_id).await
    }

    async fn reference_media_provider_request_body_for_preview(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError> {
        self.provider_request_body(input).await
    }

    async fn upload_reference_media_input_for_preview(
        &self,
        owner_user_id: i64,
        project_id: i64,
        source_media_id: i64,
        source_media_kind: &str,
        file_name: &str,
        mime_type: &str,
        bytes: &[u8],
    ) -> Result<(String, GenerationArtifactRef), AppError> {
        super::super::preview::upload_reference_media_input(
            owner_user_id,
            project_id,
            source_media_id,
            source_media_kind,
            file_name,
            mime_type,
            bytes,
        )
        .await
    }
}

#[async_trait]
impl ReferenceMediaVideoGenerationHostPort for RuntimeVideoProviderHostPort<'_> {
    async fn resolve_video_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError> {
        self.resolve_config(config_id).await
    }

    async fn reference_media_provider_request_body(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError> {
        <Self as ReferenceMediaVideoPreviewHostPort>::reference_media_provider_request_body_for_preview(
            self, input,
        )
        .await
    }

    async fn submit_reference_media_video(
        &self,
        input: VideoProviderRequestInput,
        progress: &mut dyn VideoGenerationProgressPort,
    ) -> Result<VideoGenerationResult, AppError> {
        self.submit_video(input, progress).await
    }

    async fn upload_reference_media_input(
        &self,
        owner_user_id: i64,
        project_id: i64,
        source_media_id: i64,
        source_media_kind: &str,
        file_name: &str,
        mime_type: &str,
        bytes: &[u8],
    ) -> Result<(String, GenerationArtifactRef), AppError> {
        <Self as ReferenceMediaVideoPreviewHostPort>::upload_reference_media_input_for_preview(
            self,
            owner_user_id,
            project_id,
            source_media_id,
            source_media_kind,
            file_name,
            mime_type,
            bytes,
        )
        .await
    }

    fn runtime_config_snapshot(&self) -> Option<Value> {
        self.redacted_runtime_config_snapshot()
    }

    fn video_provider_host_port(&self) -> &dyn VideoProviderHostPort {
        self
    }
}
