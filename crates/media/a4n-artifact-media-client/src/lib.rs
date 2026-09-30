use a4n_artifact_media_contract::ARTIFACT_MEDIA_WORKER_ID;
use a4n_worker_client::{
    HttpWorkerClient, WorkerCapabilities, WorkerClientConfig, WorkerClientError, WorkerEnvelope,
    WorkerError, WorkerLease, WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind,
    WorkerResponse,
};
use a4n_worker_contract::WORKER_SCHEMA_VERSION;
use chrono::{Duration as ChronoDuration, Utc};
use serde::de::DeserializeOwned;
use uuid::Uuid;

mod cleanup;
mod image_references;
mod images;
mod publication;

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

fn app_error(error: ArtifactMediaClientError) -> a4n_error::AppError {
    a4n_error::AppError::ConfigError(error.to_string())
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
