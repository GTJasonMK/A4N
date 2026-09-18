use a4n_artifact_media_contract::{
    ARTIFACT_MEDIA_MAINTENANCE_COMMAND, ARTIFACT_MEDIA_PATH_PREFIX_REFERENCED_COMMAND,
    ARTIFACT_MEDIA_PROJECT_PUBLICATION_COMMAND, ARTIFACT_MEDIA_PROTECTED_PATHS_COMMAND,
    ARTIFACT_MEDIA_UNREFERENCED_PATHS_COMMAND, ARTIFACT_MEDIA_WORKER_ID,
    ArtifactMediaMaintenanceReport, ArtifactMediaMaintenanceRequest, MediaPathPrefixRequest,
    MediaPathsRequest, ProjectPathsRequest, ProjectPublicationRequest, PublishImageRequest,
    PublishedImageResponse,
};
use a4n_worker_client::{
    HttpWorkerClient, WorkerCapabilities, WorkerClientConfig, WorkerClientError, WorkerEnvelope,
    WorkerError, WorkerLease, WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind,
    WorkerResponse,
};
use a4n_worker_contract::WORKER_SCHEMA_VERSION;
use chrono::{Duration as ChronoDuration, Utc};
use serde::de::DeserializeOwned;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ArtifactMediaClientError {
    #[error(transparent)]
    Worker(#[from] WorkerClientError),
    #[error("artifact media request is invalid: {0}")]
    InvalidRequest(String),
    #[error("artifact media worker response did not contain a completed payload")]
    MissingPayload,
    #[error("artifact media operation failed: {0:?}")]
    OperationFailed(WorkerError),
    #[error("artifact media operation returned unexpected status: {0:?}")]
    UnexpectedStatus(WorkerOperationStatus),
}

#[derive(Clone)]
pub struct ArtifactMediaWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for ArtifactMediaWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ArtifactMediaWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
}

impl ArtifactMediaWorkerClient {
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self::with_config(base_url, token, WorkerClientConfig::default())
    }

    pub fn with_config(
        base_url: impl Into<String>,
        token: impl Into<String>,
        config: WorkerClientConfig,
    ) -> Self {
        Self {
            transport: HttpWorkerClient::with_config(base_url, ARTIFACT_MEDIA_WORKER_ID, config)
                .with_auth_token(token),
        }
    }

    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var(a4n_artifact_media_contract::ARTIFACT_MEDIA_WORKER_URL_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())?;
        let token = std::env::var(a4n_artifact_media_contract::ARTIFACT_MEDIA_WORKER_TOKEN_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())?;
        Some(Self::new(base_url, token))
    }

    pub async fn health(
        &self,
    ) -> Result<a4n_worker_client::WorkerHealth, ArtifactMediaClientError> {
        Ok(self.transport.health().await?)
    }

    pub async fn capabilities(&self) -> Result<WorkerCapabilities, ArtifactMediaClientError> {
        Ok(self.transport.capabilities().await?)
    }

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

    fn envelope<T>(&self, operation_id: &str, payload: T) -> WorkerEnvelope<T> {
        self.envelope_with_principal(
            operation_id,
            WorkerPrincipal {
                kind: WorkerPrincipalKind::Service,
                subject: "a4n-runtime".to_string(),
            },
            payload,
        )
    }

    fn user_envelope<T>(&self, operation_id: &str, user_id: i64, payload: T) -> WorkerEnvelope<T> {
        self.envelope_with_principal(
            operation_id,
            WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: user_id.to_string(),
            },
            payload,
        )
    }

    fn envelope_with_principal<T>(
        &self,
        operation_id: &str,
        principal: WorkerPrincipal,
        payload: T,
    ) -> WorkerEnvelope<T> {
        let operation_id = operation_id.to_string();
        WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: Uuid::new_v4().to_string(),
            trace_id: Uuid::new_v4().to_string(),
            principal,
            operation_id: operation_id.clone(),
            idempotency_key: operation_id,
            project_id: None,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: ARTIFACT_MEDIA_WORKER_ID.to_string(),
                epoch: 1,
                fence_token: Uuid::new_v4().to_string(),
                expires_at: (Utc::now() + ChronoDuration::minutes(5)).to_rfc3339(),
            },
            payload,
        }
    }

    async fn completed_payload<I, O>(
        &self,
        command: &str,
        request: WorkerEnvelope<I>,
    ) -> Result<O, ArtifactMediaClientError>
    where
        I: serde::Serialize,
        O: DeserializeOwned,
    {
        let response: WorkerResponse<O> = self.transport.command(command, request).await?;
        match response.status {
            WorkerOperationStatus::Completed => response
                .payload
                .ok_or(ArtifactMediaClientError::MissingPayload),
            WorkerOperationStatus::Failed => Err(ArtifactMediaClientError::OperationFailed(
                response
                    .error
                    .ok_or(ArtifactMediaClientError::MissingPayload)?,
            )),
            status => Err(ArtifactMediaClientError::UnexpectedStatus(status)),
        }
    }
}

#[async_trait::async_trait]
impl a4n_artifact_media_contract::ArtifactMediaPublicationPort for ArtifactMediaWorkerClient {
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

#[async_trait::async_trait]
impl a4n_artifact_media_contract::ArtifactMediaReferencePort for ArtifactMediaWorkerClient {
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

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{WorkerHealth, WorkerHealthStatus};
    use axum::{Router, routing::get};
    use std::net::SocketAddr;
    use std::time::Duration;

    #[tokio::test]
    async fn client_uses_worker_health_contract() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address: SocketAddr = listener.local_addr().unwrap();
        let app = Router::new().route(
            "/healthz",
            get(|| async {
                axum::Json(WorkerHealth {
                    schema_version: WORKER_SCHEMA_VERSION,
                    worker_id: ARTIFACT_MEDIA_WORKER_ID.to_string(),
                    status: WorkerHealthStatus::Healthy,
                    version: env!("CARGO_PKG_VERSION").to_string(),
                })
            }),
        );
        let server = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        let client = ArtifactMediaWorkerClient::with_config(
            format!("http://{address}"),
            "test-token",
            WorkerClientConfig::new(Duration::from_secs(1), Duration::from_secs(1)),
        );
        assert!(client.health().await.is_ok());
        server.abort();
    }
}
