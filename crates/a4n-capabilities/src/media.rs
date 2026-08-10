use super::contracts::{
    CapabilityClient, CapabilityResult, ImageClient, JsonRequest, JsonResponse, VideoClient,
};
use super::transport::{HttpClientFactory, HttpSidecarTransport};
use a4n_service_protocol::{
    CapabilityTaskEvent, IMAGE_SERVICE_NAME, ServiceHealth, VIDEO_SERVICE_NAME,
};
use async_trait::async_trait;
#[cfg(any(test, feature = "test-support"))]
use std::time::Duration;

pub struct HttpImageClient {
    pub(super) transport: HttpSidecarTransport,
}

impl HttpImageClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url)
                .with_expected_service(IMAGE_SERVICE_NAME),
        }
    }

    pub fn new_with_auth_token(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url)
                .with_expected_service(IMAGE_SERVICE_NAME)
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
impl CapabilityClient for HttpImageClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
impl ImageClient for HttpImageClient {
    async fn generate_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> CapabilityResult<()> {
        self.transport
            .post_event_stream("/image/generate/events", request, events)
            .await
    }

    async fn preview_request(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/image/request_preview", request)
            .await
    }

    async fn probe(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/image/probe", request).await
    }

    async fn discover_models(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/image/models", request).await
    }
}

pub struct HttpVideoClient {
    pub(super) transport: HttpSidecarTransport,
}

impl HttpVideoClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url)
                .with_expected_service(VIDEO_SERVICE_NAME),
        }
    }

    pub fn new_with_auth_token(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url)
                .with_expected_service(VIDEO_SERVICE_NAME)
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
impl CapabilityClient for HttpVideoClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
impl VideoClient for HttpVideoClient {
    async fn generate_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> CapabilityResult<()> {
        self.transport
            .post_event_stream("/video/generate/events", request, events)
            .await
    }

    async fn poll(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/video/poll", request).await
    }

    async fn status(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/video/status", request).await
    }

    async fn delete(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/video/delete", request).await
    }

    async fn preview_request(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/video/request_preview", request)
            .await
    }

    async fn probe(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/video/probe", request).await
    }

    async fn discover_models(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/video/models", request).await
    }
}
