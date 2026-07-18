use super::contracts::{
    CapabilityClient, CapabilityResult, EmbeddingClient, JsonRequest, JsonResponse,
};
use super::transport::HttpSidecarTransport;
use a4n_service_protocol::{
    EMBEDDING_SERVICE_NAME, ServiceError, ServiceErrorCode, ServiceHealth, ServiceTaskEvent,
};
use async_trait::async_trait;

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
        events: tokio::sync::mpsc::UnboundedSender<ServiceTaskEvent>,
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
