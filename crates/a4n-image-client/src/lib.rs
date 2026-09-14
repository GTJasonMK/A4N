//! Typed client for the image sidecar service.

use a4n_image_contract::{IMAGE_PROVIDER_CAPABILITY_VERSION, IMAGE_SERVICE_NAME};
use a4n_service_client::{
    HttpClientFactory, HttpSidecarTransport, ServiceClient, ServiceClientConfig,
    ServiceClientResult,
};
use a4n_service_protocol::{
    CapabilityTaskEvent, ServiceHealth, ServiceRequest, ServiceResponse, sidecar_resource_policy,
};
use async_trait::async_trait;
#[cfg(any(test, feature = "test-support"))]
use std::time::Duration;

pub type JsonRequest = ServiceRequest<serde_json::Value>;
pub type JsonResponse = ServiceResponse<serde_json::Value>;

fn service_client_config() -> ServiceClientConfig {
    ServiceClientConfig::new(
        IMAGE_SERVICE_NAME,
        IMAGE_PROVIDER_CAPABILITY_VERSION,
        sidecar_resource_policy(IMAGE_SERVICE_NAME)
            .expect("image service must have a resource policy"),
    )
}

pub struct HttpImageClient {
    transport: HttpSidecarTransport,
}

impl HttpImageClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url, service_client_config()),
        }
    }

    pub fn new_with_auth_token(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url, service_client_config())
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
impl ServiceClient for HttpImageClient {
    async fn health(&self) -> ServiceClientResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
pub trait ImageClient: ServiceClient {
    async fn generate_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> ServiceClientResult<()>;
    async fn preview_request(&self, request: JsonRequest) -> ServiceClientResult<JsonResponse>;
    async fn discover_models(&self, request: JsonRequest) -> ServiceClientResult<JsonResponse>;
}

#[async_trait]
impl ImageClient for HttpImageClient {
    async fn generate_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> ServiceClientResult<()> {
        self.transport
            .post_event_stream("/image/generate/events", request, events)
            .await
    }

    async fn preview_request(&self, request: JsonRequest) -> ServiceClientResult<JsonResponse> {
        self.transport.post("/image/request_preview", request).await
    }

    async fn discover_models(&self, request: JsonRequest) -> ServiceClientResult<JsonResponse> {
        self.transport.post("/image/models", request).await
    }
}
