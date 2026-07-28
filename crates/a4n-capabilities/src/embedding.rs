use super::contracts::{
    CapabilityClient, CapabilityResult, EmbeddingClient, JsonRequest, JsonResponse,
};
use super::transport::{HttpClientFactory, HttpSidecarTransport};
use a4n_service_protocol::{
    CapabilityTaskEvent, EMBEDDING_SERVICE_NAME, ServiceError, ServiceErrorCode, ServiceHealth,
};
use async_trait::async_trait;
#[cfg(any(test, feature = "test-support"))]
use std::time::Duration;

pub struct HttpEmbeddingClient {
    pub(super) transport: HttpSidecarTransport,
}

impl HttpEmbeddingClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url)
                .with_expected_service(EMBEDDING_SERVICE_NAME),
        }
    }

    pub fn new_with_auth_token(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url)
                .with_expected_service(EMBEDDING_SERVICE_NAME)
                .with_auth_token(token),
        }
    }

    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.transport = self.transport.with_http_client(client);
        self
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
impl CapabilityClient for HttpEmbeddingClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
impl EmbeddingClient for HttpEmbeddingClient {
    async fn embed(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/embedding/embed_texts", request)
            .await
    }

    async fn model_operation(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        let operation = request
            .payload
            .get("operation")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let path = embedding_model_operation_path(operation)?;
        self.transport.post_json(path, request).await
    }

    async fn download_model_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> CapabilityResult<()> {
        self.transport
            .post_event_stream(EMBEDDING_MODEL_DOWNLOAD_EVENTS_PATH, request, events)
            .await
    }
}

pub(super) const EMBEDDING_MODEL_DOWNLOAD_EVENTS_PATH: &str = "/embedding/model/download/events";

pub(super) fn embedding_model_operation_path(operation: &str) -> CapabilityResult<&'static str> {
    match operation {
        "list" => Ok("/embedding/models"),
        "download_info" => Ok("/embedding/model/download_info"),
        "download" => Ok("/embedding/model/download"),
        "delete" => Ok("/embedding/model/delete"),
        _ => Err(ServiceError {
            code: ServiceErrorCode::ConfigurationError,
            message: format!("unsupported embedding model operation: {operation}"),
            retryable: Some(false),
        }),
    }
}
