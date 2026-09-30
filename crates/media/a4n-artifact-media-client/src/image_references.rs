use super::{ArtifactMediaClientError, ArtifactMediaWorkerClient, app_error};
use a4n_api_contract::image::{GeneratedImageSummary, ImageReferenceReadApplicationPort};
use a4n_artifact_media_contract::{
    ARTIFACT_MEDIA_IMAGE_REFERENCE_IDS_COMMAND, ImageReferenceIdsRequest,
};

impl ArtifactMediaWorkerClient {
    pub async fn image_reference_by_ids(
        &self,
        actor_user_id: i64,
        project_id: i64,
        image_ids: &[i64],
    ) -> Result<Vec<GeneratedImageSummary>, ArtifactMediaClientError> {
        self.completed_payload(
            ARTIFACT_MEDIA_IMAGE_REFERENCE_IDS_COMMAND,
            self.user_envelope(
                "artifact-media.image-reference-ids",
                actor_user_id,
                serde_json::to_value(ImageReferenceIdsRequest {
                    project_id,
                    image_ids: image_ids.to_vec(),
                })
                .expect("image reference input serializes"),
            ),
        )
        .await
    }
}

#[async_trait::async_trait]
impl ImageReferenceReadApplicationPort for ArtifactMediaWorkerClient {
    async fn reference_by_ids(
        &self,
        user_id: i64,
        project_id: i64,
        image_ids: &[i64],
    ) -> Result<Vec<GeneratedImageSummary>, a4n_error::AppError> {
        self.image_reference_by_ids(user_id, project_id, image_ids)
            .await
            .map_err(app_error)
    }
}
