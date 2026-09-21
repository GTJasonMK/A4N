//! Typed client for the local media worker.

use a4n_media_contract::{
    MEDIA_FFMPEG_CAPABILITY_VERSION, MEDIA_SERVICE_NAME, MediaExecuteInput, MediaExecuteOutput,
};
use a4n_service_client::{
    HttpClientFactory, HttpSidecarTransport, ServiceClient, ServiceClientConfig,
    ServiceClientResult,
};
use a4n_service_protocol::{
    ServiceHealth, ServiceRequest, ServiceResponse, sidecar_resource_policy,
};
use async_trait::async_trait;

fn service_client_config() -> ServiceClientConfig {
    ServiceClientConfig::new(
        MEDIA_SERVICE_NAME,
        MEDIA_FFMPEG_CAPABILITY_VERSION,
        sidecar_resource_policy(MEDIA_SERVICE_NAME)
            .expect("media service must have a resource policy"),
    )
}

#[derive(Debug, Clone)]
pub struct HttpMediaClient {
    transport: HttpSidecarTransport,
}

impl HttpMediaClient {
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
}

#[async_trait]
impl ServiceClient for HttpMediaClient {
    async fn health(&self) -> ServiceClientResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
pub trait MediaClient: ServiceClient {
    async fn execute(
        &self,
        request: ServiceRequest<MediaExecuteInput>,
    ) -> ServiceClientResult<ServiceResponse<MediaExecuteOutput>>;
}

#[async_trait]
impl MediaClient for HttpMediaClient {
    async fn execute(
        &self,
        request: ServiceRequest<MediaExecuteInput>,
    ) -> ServiceClientResult<ServiceResponse<MediaExecuteOutput>> {
        self.transport.post("/media/execute", request).await
    }
}
