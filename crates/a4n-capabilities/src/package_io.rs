use super::contracts::{CapabilityClient, CapabilityResult, ExportClient, ImportClient};
use super::transport::{HttpClientFactory, HttpSidecarTransport};
use a4n_export_contract::{ExportRenderJsonInput, ExportRenderZipInput, ExportRenderZipOutput};
use a4n_import_contract::{ImportReadMediaPackageZipInput, ImportReadMediaPackageZipOutput};
use a4n_service_protocol::{
    EXPORT_SERVICE_NAME, IMPORT_SERVICE_NAME, ServiceHealth, ServiceRequest, ServiceResponse,
};
use async_trait::async_trait;

pub struct HttpImportClient {
    transport: HttpSidecarTransport,
}

impl HttpImportClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url, IMPORT_SERVICE_NAME),
        }
    }

    pub fn new_with_auth_token(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url, IMPORT_SERVICE_NAME)
                .with_auth_token(token),
        }
    }

    pub fn with_http_client_factory(mut self, factory: HttpClientFactory) -> Self {
        self.transport = self.transport.with_http_client_factory(factory);
        self
    }
}

#[async_trait]
impl CapabilityClient for HttpImportClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
impl ImportClient for HttpImportClient {
    async fn read_media_package_zip(
        &self,
        request: ServiceRequest<ImportReadMediaPackageZipInput>,
    ) -> CapabilityResult<ServiceResponse<ImportReadMediaPackageZipOutput>> {
        self.transport
            .post("/import/read_media_package_zip", request)
            .await
    }
}

#[derive(Debug, Clone)]
pub struct HttpExportClient {
    transport: HttpSidecarTransport,
}

impl HttpExportClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url, EXPORT_SERVICE_NAME),
        }
    }

    pub fn new_with_auth_token(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url, EXPORT_SERVICE_NAME)
                .with_auth_token(token),
        }
    }

    pub fn with_http_client_factory(mut self, factory: HttpClientFactory) -> Self {
        self.transport = self.transport.with_http_client_factory(factory);
        self
    }
}

#[async_trait]
impl CapabilityClient for HttpExportClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
impl ExportClient for HttpExportClient {
    async fn render_json(
        &self,
        request: ServiceRequest<ExportRenderJsonInput>,
    ) -> CapabilityResult<ServiceResponse<String>> {
        self.transport.post("/export/render_json", request).await
    }

    async fn render_zip(
        &self,
        request: ServiceRequest<ExportRenderZipInput>,
    ) -> CapabilityResult<ServiceResponse<ExportRenderZipOutput>> {
        self.transport.post("/export/render_zip", request).await
    }
}
