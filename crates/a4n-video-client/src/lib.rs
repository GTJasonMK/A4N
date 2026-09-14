//! Typed client for the video sidecar service.

use a4n_model_discovery_contract::{ModelDiscoveryRequest, ModelDiscoveryResult};
use a4n_service_client::{
    HttpClientFactory, HttpSidecarTransport, ServiceClient, ServiceClientConfig,
    ServiceClientResult,
};
use a4n_service_protocol::{
    CapabilityTaskEvent, ServiceHealth, ServiceRequest, ServiceResponse, VIDEO_SERVICE_NAME,
};
use async_trait::async_trait;
#[cfg(any(test, feature = "test-support"))]
use std::time::Duration;

pub type JsonRequest = ServiceRequest<serde_json::Value>;
pub type JsonResponse = ServiceResponse<serde_json::Value>;

pub struct HttpVideoClient {
    transport: HttpSidecarTransport,
}

impl HttpVideoClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(
                base_url,
                ServiceClientConfig::for_service(VIDEO_SERVICE_NAME),
            ),
        }
    }

    pub fn new_with_auth_token(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(
                base_url,
                ServiceClientConfig::for_service(VIDEO_SERVICE_NAME),
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
impl ServiceClient for HttpVideoClient {
    async fn health(&self) -> ServiceClientResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
pub trait VideoClient: ServiceClient {
    async fn generate_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> ServiceClientResult<()>;
    async fn poll(&self, request: JsonRequest) -> ServiceClientResult<JsonResponse>;
    async fn status(&self, request: JsonRequest) -> ServiceClientResult<JsonResponse>;
    async fn delete(&self, request: JsonRequest) -> ServiceClientResult<JsonResponse>;
    async fn preview_request(&self, request: JsonRequest) -> ServiceClientResult<JsonResponse>;
    async fn discover_models(
        &self,
        request: ServiceRequest<ModelDiscoveryRequest>,
    ) -> ServiceClientResult<ServiceResponse<ModelDiscoveryResult>>;
}

#[async_trait]
impl VideoClient for HttpVideoClient {
    async fn generate_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> ServiceClientResult<()> {
        self.transport
            .post_event_stream("/video/generate/events", request, events)
            .await
    }

    async fn poll(&self, request: JsonRequest) -> ServiceClientResult<JsonResponse> {
        self.transport.post("/video/poll", request).await
    }

    async fn status(&self, request: JsonRequest) -> ServiceClientResult<JsonResponse> {
        self.transport.post("/video/status", request).await
    }

    async fn delete(&self, request: JsonRequest) -> ServiceClientResult<JsonResponse> {
        self.transport.post("/video/delete", request).await
    }

    async fn preview_request(&self, request: JsonRequest) -> ServiceClientResult<JsonResponse> {
        self.transport.post("/video/request_preview", request).await
    }

    async fn discover_models(
        &self,
        request: ServiceRequest<ModelDiscoveryRequest>,
    ) -> ServiceClientResult<ServiceResponse<ModelDiscoveryResult>> {
        self.transport.post("/video/models", request).await
    }
}
