use super::{ArtifactMediaWorkerClient, app_error};
use a4n_api_contract::image::{GeneratedImageSummary, ImageReferenceReadApplicationPort};
use a4n_artifact_media_contract::{
    ArtifactMediaReferencePort, MediaPathPrefixRequest, MediaPathsRequest, ProjectPathsRequest,
    ProjectPublicationRequest,
};

#[async_trait::async_trait]
impl ArtifactMediaReferencePort for ArtifactMediaWorkerClient {
    async fn unreferenced_paths(&self, request: MediaPathsRequest) -> Result<Vec<String>, String> {
        ArtifactMediaWorkerClient::unreferenced_paths(self, request)
            .await
            .map_err(|error| error.to_string())
    }

    async fn path_prefix_has_reference(
        &self,
        request: MediaPathPrefixRequest,
    ) -> Result<bool, String> {
        ArtifactMediaWorkerClient::path_prefix_has_reference(self, request)
            .await
            .map_err(|error| error.to_string())
    }

    async fn project_has_publication(
        &self,
        request: ProjectPublicationRequest,
    ) -> Result<bool, String> {
        ArtifactMediaWorkerClient::project_has_publication(self, request)
            .await
            .map_err(|error| error.to_string())
    }

    async fn protected_paths(&self, request: ProjectPathsRequest) -> Result<Vec<String>, String> {
        ArtifactMediaWorkerClient::protected_paths(self, request)
            .await
            .map_err(|error| error.to_string())
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
