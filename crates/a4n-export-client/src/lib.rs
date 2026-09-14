//! Typed client for the export sidecar service.

use a4n_export_contract::{
    EXPORT_RENDER_CAPABILITY_VERSION, EXPORT_SERVICE_NAME, ExportRenderJsonInput,
    ExportRenderZipInput, ExportRenderZipOutput,
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
        EXPORT_SERVICE_NAME,
        EXPORT_RENDER_CAPABILITY_VERSION,
        sidecar_resource_policy(EXPORT_SERVICE_NAME)
            .expect("export service must have a resource policy"),
    )
}

#[derive(Debug, Clone)]
pub struct HttpExportClient {
    transport: HttpSidecarTransport,
}

impl HttpExportClient {
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
impl ServiceClient for HttpExportClient {
    async fn health(&self) -> ServiceClientResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
pub trait ExportClient: ServiceClient {
    async fn render_json(
        &self,
        request: ServiceRequest<ExportRenderJsonInput>,
    ) -> ServiceClientResult<ServiceResponse<String>>;
    async fn render_zip(
        &self,
        request: ServiceRequest<ExportRenderZipInput>,
    ) -> ServiceClientResult<ServiceResponse<ExportRenderZipOutput>>;
}

#[async_trait]
impl ExportClient for HttpExportClient {
    async fn render_json(
        &self,
        request: ServiceRequest<ExportRenderJsonInput>,
    ) -> ServiceClientResult<ServiceResponse<String>> {
        self.transport.post("/export/render_json", request).await
    }

    async fn render_zip(
        &self,
        request: ServiceRequest<ExportRenderZipInput>,
    ) -> ServiceClientResult<ServiceResponse<ExportRenderZipOutput>> {
        self.transport.post("/export/render_zip", request).await
    }
}
