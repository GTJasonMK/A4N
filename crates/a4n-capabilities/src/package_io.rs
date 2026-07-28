use super::contracts::{
    CapabilityClient, CapabilityResult, ExportClient, ImportClient, JsonRequest, JsonResponse,
};
use super::transport::{HttpClientFactory, HttpSidecarTransport};
use a4n_export_core::{
    ExportRenderJsonInput, ExportRenderJsonOutput, ExportRenderZipInput, ExportRenderZipOutput,
};
use a4n_import_core::{
    ImportNormalizeJsonInput, ImportNormalizeJsonOutput, ImportReadMediaPackageZipInput,
    ImportReadMediaPackageZipOutput,
};
use a4n_service_protocol::{
    EXPORT_SERVICE_NAME, IMPORT_SERVICE_NAME, ServiceError, ServiceErrorCode, ServiceHealth,
    ServiceRequest, ServiceResponse,
};
use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};

pub struct HttpImportClient {
    pub(super) transport: HttpSidecarTransport,
}

impl HttpImportClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url)
                .with_expected_service(IMPORT_SERVICE_NAME),
        }
    }

    pub fn new_with_auth_token(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url)
                .with_expected_service(IMPORT_SERVICE_NAME)
                .with_auth_token(token),
        }
    }

    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.transport = self.transport.with_http_client(client);
        self
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
    async fn normalize_json(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/import/normalize_json", request)
            .await
    }

    async fn read_media_package_zip(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/import/read_media_package_zip", request)
            .await
    }

    async fn normalize_json_typed(
        &self,
        request: ServiceRequest<ImportNormalizeJsonInput>,
    ) -> CapabilityResult<ServiceResponse<ImportNormalizeJsonOutput>> {
        let response = self.normalize_json(into_json_request(request)?).await?;
        from_json_response(response)
    }

    async fn read_media_package_zip_typed(
        &self,
        request: ServiceRequest<ImportReadMediaPackageZipInput>,
    ) -> CapabilityResult<ServiceResponse<ImportReadMediaPackageZipOutput>> {
        let response = self
            .read_media_package_zip(into_json_request(request)?)
            .await?;
        from_json_response(response)
    }
}

#[derive(Debug, Clone)]
pub struct HttpExportClient {
    pub(super) transport: HttpSidecarTransport,
}

impl HttpExportClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url)
                .with_expected_service(EXPORT_SERVICE_NAME),
        }
    }

    pub fn new_with_auth_token(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url)
                .with_expected_service(EXPORT_SERVICE_NAME)
                .with_auth_token(token),
        }
    }

    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.transport = self.transport.with_http_client(client);
        self
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
    async fn render_json(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/export/render_json", request)
            .await
    }

    async fn render_zip(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/export/render_zip", request)
            .await
    }

    async fn render_json_typed(
        &self,
        request: ServiceRequest<ExportRenderJsonInput>,
    ) -> CapabilityResult<ServiceResponse<ExportRenderJsonOutput>> {
        let response = self.render_json(into_json_request(request)?).await?;
        from_json_response(response)
    }

    async fn render_zip_typed(
        &self,
        request: ServiceRequest<ExportRenderZipInput>,
    ) -> CapabilityResult<ServiceResponse<ExportRenderZipOutput>> {
        let response = self.render_zip(into_json_request(request)?).await?;
        from_json_response(response)
    }
}

pub(super) fn into_json_request<T>(request: ServiceRequest<T>) -> CapabilityResult<JsonRequest>
where
    T: Serialize,
{
    let (context, capability_version, payload) = request.into_context_and_payload();
    let payload = serde_json::to_value(payload).map_err(typed_request_error)?;
    context.with_payload(capability_version, payload)
}

pub(super) fn from_json_response<T>(response: JsonResponse) -> CapabilityResult<ServiceResponse<T>>
where
    T: DeserializeOwned,
{
    let ServiceResponse {
        status,
        trace_id,
        usage,
        artifact_refs,
        payload,
        error,
    } = response;
    let payload = payload
        .map(serde_json::from_value)
        .transpose()
        .map_err(typed_response_error)?;
    Ok(ServiceResponse {
        status,
        trace_id,
        usage,
        artifact_refs,
        payload,
        error,
    })
}

pub(super) fn typed_request_error(error: serde_json::Error) -> ServiceError {
    ServiceError {
        code: ServiceErrorCode::ConfigurationError,
        message: format!("typed sidecar request payload serialization failed: {error}"),
        retryable: Some(false),
    }
}

pub(super) fn typed_response_error(error: serde_json::Error) -> ServiceError {
    ServiceError {
        code: ServiceErrorCode::Internal,
        message: format!("typed sidecar response payload deserialization failed: {error}"),
        retryable: Some(false),
    }
}
