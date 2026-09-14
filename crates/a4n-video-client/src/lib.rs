//! Typed client for the video sidecar service.

use a4n_model_discovery_contract::{ModelDiscoveryRequest, ModelDiscoveryResult};
use a4n_service_client::{
    HttpClientFactory, HttpSidecarTransport, ServiceClient, ServiceClientConfig,
    ServiceClientResult,
};
use a4n_service_protocol::{CapabilityTaskEvent, ServiceHealth, ServiceRequest, ServiceResponse};
use a4n_video_sidecar_contract::{
    VIDEO_PROVIDER_CAPABILITY_VERSION, VIDEO_SERVICE_NAME, VideoDeleteOutput, VideoJobOutput,
    VideoPollPayload, VideoProviderPayload, VideoStatusOutput,
};
use async_trait::async_trait;
#[cfg(any(test, feature = "test-support"))]
use std::time::Duration;

pub type JsonResponse = ServiceResponse<serde_json::Value>;
pub type VideoProviderRequest = ServiceRequest<VideoProviderPayload>;
pub type VideoPollRequest = ServiceRequest<VideoPollPayload>;
pub type VideoJobResponse = ServiceResponse<VideoJobOutput>;
pub type VideoStatusResponse = ServiceResponse<VideoStatusOutput>;
pub type VideoDeleteResponse = ServiceResponse<VideoDeleteOutput>;

pub struct HttpVideoClient {
    transport: HttpSidecarTransport,
}

impl HttpVideoClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url, video_service_client_config()),
        }
    }

    pub fn new_with_auth_token(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url, video_service_client_config())
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

fn video_service_client_config() -> ServiceClientConfig {
    ServiceClientConfig::new(
        VIDEO_SERVICE_NAME,
        VIDEO_PROVIDER_CAPABILITY_VERSION,
        a4n_service_protocol::sidecar_resource_policy(VIDEO_SERVICE_NAME)
            .expect("video service must have a resource policy"),
    )
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
        request: VideoProviderRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> ServiceClientResult<()>;
    async fn poll(&self, request: VideoPollRequest) -> ServiceClientResult<VideoJobResponse>;
    async fn status(&self, request: VideoPollRequest) -> ServiceClientResult<VideoStatusResponse>;
    async fn delete(&self, request: VideoPollRequest) -> ServiceClientResult<VideoDeleteResponse>;
    async fn preview_request(
        &self,
        request: VideoProviderRequest,
    ) -> ServiceClientResult<JsonResponse>;
    async fn discover_models(
        &self,
        request: ServiceRequest<ModelDiscoveryRequest>,
    ) -> ServiceClientResult<ServiceResponse<ModelDiscoveryResult>>;
}

#[async_trait]
impl VideoClient for HttpVideoClient {
    async fn generate_events(
        &self,
        request: VideoProviderRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> ServiceClientResult<()> {
        self.transport
            .post_event_stream("/video/generate/events", request, events)
            .await
    }

    async fn poll(&self, request: VideoPollRequest) -> ServiceClientResult<VideoJobResponse> {
        self.transport.post("/video/poll", request).await
    }

    async fn status(&self, request: VideoPollRequest) -> ServiceClientResult<VideoStatusResponse> {
        self.transport.post("/video/status", request).await
    }

    async fn delete(&self, request: VideoPollRequest) -> ServiceClientResult<VideoDeleteResponse> {
        self.transport.post("/video/delete", request).await
    }

    async fn preview_request(
        &self,
        request: VideoProviderRequest,
    ) -> ServiceClientResult<JsonResponse> {
        self.transport.post("/video/request_preview", request).await
    }

    async fn discover_models(
        &self,
        request: ServiceRequest<ModelDiscoveryRequest>,
    ) -> ServiceClientResult<ServiceResponse<ModelDiscoveryResult>> {
        self.transport.post("/video/models", request).await
    }
}
