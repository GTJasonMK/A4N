//! Typed client for the import sidecar service.

use a4n_import_contract::{
    IMPORT_SERVICE_NAME, IMPORT_TRANSFORM_CAPABILITY_VERSION, ImportParseDocumentInput,
    ImportParseDocumentOutput, ImportReadMediaPackageZipInput, ImportReadMediaPackageZipOutput,
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
        IMPORT_SERVICE_NAME,
        IMPORT_TRANSFORM_CAPABILITY_VERSION,
        sidecar_resource_policy(IMPORT_SERVICE_NAME)
            .expect("import service must have a resource policy"),
    )
}

pub struct HttpImportClient {
    transport: HttpSidecarTransport,
}

impl HttpImportClient {
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
impl ServiceClient for HttpImportClient {
    async fn health(&self) -> ServiceClientResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
pub trait ImportClient: ServiceClient {
    async fn parse_document(
        &self,
        request: ServiceRequest<ImportParseDocumentInput>,
    ) -> ServiceClientResult<ServiceResponse<ImportParseDocumentOutput>>;
    async fn read_media_package_zip(
        &self,
        request: ServiceRequest<ImportReadMediaPackageZipInput>,
    ) -> ServiceClientResult<ServiceResponse<ImportReadMediaPackageZipOutput>>;
}

#[async_trait]
impl ImportClient for HttpImportClient {
    async fn parse_document(
        &self,
        request: ServiceRequest<ImportParseDocumentInput>,
    ) -> ServiceClientResult<ServiceResponse<ImportParseDocumentOutput>> {
        self.transport.post("/import/parse_document", request).await
    }
    async fn read_media_package_zip(
        &self,
        request: ServiceRequest<ImportReadMediaPackageZipInput>,
    ) -> ServiceClientResult<ServiceResponse<ImportReadMediaPackageZipOutput>> {
        self.transport
            .post("/import/read_media_package_zip", request)
            .await
    }
}
