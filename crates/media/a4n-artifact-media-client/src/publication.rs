use super::ArtifactMediaClientError;
use super::ArtifactMediaWorkerClient;
use a4n_artifact_media_contract::{
    ArtifactMediaPublicationPort, PublishImageRequest, PublishedImageResponse,
};

impl ArtifactMediaWorkerClient {
    pub async fn publish_image(
        &self,
        actor_user_id: i64,
        request: PublishImageRequest,
    ) -> Result<PublishedImageResponse, ArtifactMediaClientError> {
        if actor_user_id <= 0 {
            return Err(ArtifactMediaClientError::InvalidRequest(
                "actor_user_id must be positive".to_string(),
            ));
        }
        request
            .validate()
            .map_err(|error| ArtifactMediaClientError::InvalidRequest(error.to_string()))?;
        self.completed_payload(
            a4n_artifact_media_contract::ARTIFACT_MEDIA_PUBLISH_IMAGE_COMMAND,
            self.user_envelope("artifact-media.publish-image", actor_user_id, request),
        )
        .await
    }
}

#[async_trait::async_trait]
impl ArtifactMediaPublicationPort for ArtifactMediaWorkerClient {
    async fn publish_image(
        &self,
        actor_user_id: i64,
        request: PublishImageRequest,
    ) -> Result<PublishedImageResponse, String> {
        self.publish_image(actor_user_id, request)
            .await
            .map_err(|error| error.to_string())
    }
}
