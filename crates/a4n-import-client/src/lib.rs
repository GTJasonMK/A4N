//! Typed client for the import sidecar service.

use a4n_import_contract::{ImportReadMediaPackageZipInput, ImportReadMediaPackageZipOutput};
use a4n_service_client::{
    HttpClientFactory, HttpSidecarTransport, ServiceClient, ServiceClientConfig,
    ServiceClientResult,
};
use a4n_service_protocol::{IMPORT_SERVICE_NAME, ServiceHealth, ServiceRequest, ServiceResponse};
use async_trait::async_trait;

pub struct HttpImportClient {
    transport: HttpSidecarTransport,
}

impl HttpImportClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(
                base_url,
                ServiceClientConfig::for_service(IMPORT_SERVICE_NAME),
            ),
        }
    }

    pub fn new_with_auth_token(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(
                base_url,
                ServiceClientConfig::for_service(IMPORT_SERVICE_NAME),
            )
            .with_auth_token(token),
        }
    }

    pub fn with_http_client_factory(mut self, factory: HttpClientFactory) -> Self {
        self.transport = self.transport.with_http_client_factory(factory);
        self
    }
}

#[async_trait]
impl ServiceClient for HttpImportClient {
    async fn health(&self) -> ServiceClientResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
pub trait ImportClient: ServiceClient {
    async fn read_media_package_zip(
        &self,
        request: ServiceRequest<ImportReadMediaPackageZipInput>,
    ) -> ServiceClientResult<ServiceResponse<ImportReadMediaPackageZipOutput>>;
}

#[async_trait]
impl ImportClient for HttpImportClient {
    async fn read_media_package_zip(
        &self,
        request: ServiceRequest<ImportReadMediaPackageZipInput>,
    ) -> ServiceClientResult<ServiceResponse<ImportReadMediaPackageZipOutput>> {
        self.transport
            .post("/import/read_media_package_zip", request)
            .await
    }
}
