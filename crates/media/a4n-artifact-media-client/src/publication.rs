use super::ArtifactMediaWorkerClient;
use a4n_artifact_media_contract::{
    ArtifactMediaPublicationPort, PublishImageRequest, PublishedImageResponse,
};

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
