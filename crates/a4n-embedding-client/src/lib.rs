//! Typed client for the embedding sidecar service.

use a4n_embedding_contract::{EmbedTextsOutput, EmbedTextsPayload, EmbeddingModelPayload};
use a4n_service_client::{
    HttpClientFactory, HttpSidecarTransport, ServiceClient, ServiceClientConfig,
    ServiceClientResult,
};
use a4n_service_protocol::{
    CapabilityTaskEvent, EMBEDDING_EMBED_TEXTS_PATH, EMBEDDING_MODEL_DELETE_PATH,
    EMBEDDING_MODEL_DOWNLOAD_EVENTS_PATH, EMBEDDING_MODEL_DOWNLOAD_INFO_PATH,
    EMBEDDING_MODELS_PATH, EMBEDDING_SERVICE_NAME, ServiceHealth, ServiceRequest, ServiceResponse,
};
use async_trait::async_trait;
#[cfg(any(test, feature = "test-support"))]
use std::time::Duration;

pub type JsonRequest = ServiceRequest<serde_json::Value>;
pub type JsonResponse = ServiceResponse<serde_json::Value>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingModelOperation {
    List,
    DownloadInfo,
    Delete,
}

pub struct HttpEmbeddingClient {
    transport: HttpSidecarTransport,
}

impl HttpEmbeddingClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(
                base_url,
                ServiceClientConfig::for_service(EMBEDDING_SERVICE_NAME),
            ),
        }
    }

    pub fn new_with_auth_token(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(
                base_url,
                ServiceClientConfig::for_service(EMBEDDING_SERVICE_NAME),
            )
            .with_auth_token(token),
        }
    }

    pub fn with_http_client_factory(mut self, factory: HttpClientFactory) -> Self {
        self.transport = self.transport.with_http_client_factory(factory);
        self
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn with_test_timeouts(
        mut self,
        connect_timeout: Duration,
        request_timeout: Duration,
        stream_idle_timeout: Duration,
        health_timeout: Duration,
    ) -> Self {
        self.transport = self.transport.with_test_timeouts(
            connect_timeout,
            request_timeout,
            stream_idle_timeout,
            health_timeout,
        );
        self
    }
}

#[async_trait]
impl ServiceClient for HttpEmbeddingClient {
    async fn health(&self) -> ServiceClientResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
pub trait EmbeddingClient: ServiceClient {
    async fn embed(
        &self,
        request: ServiceRequest<EmbedTextsPayload>,
    ) -> ServiceClientResult<ServiceResponse<EmbedTextsOutput>>;
    async fn model_operation(
        &self,
        operation: EmbeddingModelOperation,
        request: JsonRequest,
    ) -> ServiceClientResult<JsonResponse>;
    async fn download_model_events(
        &self,
        request: ServiceRequest<EmbeddingModelPayload>,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> ServiceClientResult<()>;
}

#[async_trait]
impl EmbeddingClient for HttpEmbeddingClient {
    async fn embed(
        &self,
        request: ServiceRequest<EmbedTextsPayload>,
    ) -> ServiceClientResult<ServiceResponse<EmbedTextsOutput>> {
        self.transport
            .post(EMBEDDING_EMBED_TEXTS_PATH, request)
            .await
    }

    async fn model_operation(
        &self,
        operation: EmbeddingModelOperation,
        request: JsonRequest,
    ) -> ServiceClientResult<JsonResponse> {
        self.transport
            .post(embedding_model_operation_path(operation), request)
            .await
    }

    async fn download_model_events(
        &self,
        request: ServiceRequest<EmbeddingModelPayload>,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> ServiceClientResult<()> {
        self.transport
            .post_event_stream(EMBEDDING_MODEL_DOWNLOAD_EVENTS_PATH, request, events)
            .await
    }
}

fn embedding_model_operation_path(operation: EmbeddingModelOperation) -> &'static str {
    match operation {
        EmbeddingModelOperation::List => EMBEDDING_MODELS_PATH,
        EmbeddingModelOperation::DownloadInfo => EMBEDDING_MODEL_DOWNLOAD_INFO_PATH,
        EmbeddingModelOperation::Delete => EMBEDDING_MODEL_DELETE_PATH,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_operation_paths_are_explicit() {
        assert_eq!(
            embedding_model_operation_path(EmbeddingModelOperation::List),
            "/embedding/models"
        );
        assert_eq!(
            embedding_model_operation_path(EmbeddingModelOperation::DownloadInfo),
            "/embedding/model/download_info"
        );
        assert_eq!(
            embedding_model_operation_path(EmbeddingModelOperation::Delete),
            "/embedding/model/delete"
        );
        assert_eq!(
            EMBEDDING_MODEL_DOWNLOAD_EVENTS_PATH,
            "/embedding/model/download/events"
        );
    }
}
