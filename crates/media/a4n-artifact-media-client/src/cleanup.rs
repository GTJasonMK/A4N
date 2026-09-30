use super::{ArtifactMediaClientError, ArtifactMediaWorkerClient};
use a4n_artifact_media_contract::{
    ARTIFACT_MEDIA_MAINTENANCE_COMMAND, ARTIFACT_MEDIA_PATH_PREFIX_REFERENCED_COMMAND,
    ARTIFACT_MEDIA_PROJECT_PUBLICATION_COMMAND, ARTIFACT_MEDIA_PROTECTED_PATHS_COMMAND,
    ARTIFACT_MEDIA_UNREFERENCED_PATHS_COMMAND, ArtifactMediaMaintenanceReport,
    ArtifactMediaMaintenanceRequest, ArtifactMediaReferencePort, MediaPathPrefixRequest,
    MediaPathsRequest, ProjectPathsRequest, ProjectPublicationRequest,
};

impl ArtifactMediaWorkerClient {
    pub async fn maintain(
        &self,
        request: ArtifactMediaMaintenanceRequest,
    ) -> Result<ArtifactMediaMaintenanceReport, ArtifactMediaClientError> {
        request
            .validate()
            .map_err(|error| ArtifactMediaClientError::InvalidRequest(error.to_string()))?;
        self.completed_payload(
            ARTIFACT_MEDIA_MAINTENANCE_COMMAND,
            self.envelope("artifact-media.maintain", request),
        )
        .await
    }

    pub async fn unreferenced_paths(
        &self,
        request: MediaPathsRequest,
    ) -> Result<Vec<String>, ArtifactMediaClientError> {
        request
            .validate()
            .map_err(|error| ArtifactMediaClientError::InvalidRequest(error.to_string()))?;
        self.completed_payload(
            ARTIFACT_MEDIA_UNREFERENCED_PATHS_COMMAND,
            self.envelope("artifact-media.unreferenced-paths", request),
        )
        .await
    }

    pub async fn path_prefix_has_reference(
        &self,
        request: MediaPathPrefixRequest,
    ) -> Result<bool, ArtifactMediaClientError> {
        request
            .validate()
            .map_err(|error| ArtifactMediaClientError::InvalidRequest(error.to_string()))?;
        self.completed_payload(
            ARTIFACT_MEDIA_PATH_PREFIX_REFERENCED_COMMAND,
            self.envelope("artifact-media.path-prefix-referenced", request),
        )
        .await
    }

    pub async fn project_has_publication(
        &self,
        request: ProjectPublicationRequest,
    ) -> Result<bool, ArtifactMediaClientError> {
        request
            .validate()
            .map_err(|error| ArtifactMediaClientError::InvalidRequest(error.to_string()))?;
        self.completed_payload(
            ARTIFACT_MEDIA_PROJECT_PUBLICATION_COMMAND,
            self.envelope("artifact-media.project-publication", request),
        )
        .await
    }

    pub async fn protected_paths(
        &self,
        request: ProjectPathsRequest,
    ) -> Result<Vec<String>, ArtifactMediaClientError> {
        request
            .validate()
            .map_err(|error| ArtifactMediaClientError::InvalidRequest(error.to_string()))?;
        self.completed_payload(
            ARTIFACT_MEDIA_PROTECTED_PATHS_COMMAND,
            self.envelope("artifact-media.protected-paths", request),
        )
        .await
    }
}

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
