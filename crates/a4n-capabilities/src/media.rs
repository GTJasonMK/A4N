use super::contracts::{
    CapabilityClient, CapabilityResult, ImageClient, JsonRequest, JsonResponse, VideoClient,
};
use super::transport::HttpSidecarTransport;
use a4n_service_protocol::{
    IMAGE_SERVICE_NAME, ServiceHealth, ServiceTaskEvent, VIDEO_SERVICE_NAME,
};
use async_trait::async_trait;

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
}

#[async_trait]
impl CapabilityClient for HttpImageClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
impl ImageClient for HttpImageClient {
    async fn generate(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/image/generate", request).await
    }

    async fn generate_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::UnboundedSender<ServiceTaskEvent>,
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
}

#[async_trait]
impl CapabilityClient for HttpVideoClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
impl VideoClient for HttpVideoClient {
    async fn generate(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/video/generate", request).await
    }

    async fn generate_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::UnboundedSender<ServiceTaskEvent>,
    ) -> CapabilityResult<()> {
        self.transport
            .post_event_stream("/video/generate/events", request, events)
            .await
    }

    async fn poll(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/video/poll", request).await
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
