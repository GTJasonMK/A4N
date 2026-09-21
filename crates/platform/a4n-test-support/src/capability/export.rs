use std::sync::Arc;

use a4n_export_client::ExportClient;
use a4n_export_contract::{
    ExportRenderDocumentInput, ExportRenderDocumentOutput, ExportRenderJsonInput,
    ExportRenderZipInput, ExportRenderZipOutput,
};
use a4n_service_client::{ServiceClient, ServiceClientResult as CapabilityResult};
use a4n_service_protocol::{ServiceHealth, ServiceRequest, ServiceResponse};
use async_trait::async_trait;

use super::{media::RequestLedger, unexpected_call};

#[derive(Clone, Default)]
pub struct RecordingExportClient {
    state: Arc<RecordingExportClientState>,
    render_zip_response: Option<ServiceResponse<ExportRenderZipOutput>>,
}

#[derive(Default)]
struct RecordingExportClientState {
    render_zip_requests: RequestLedger<ServiceRequest<ExportRenderZipInput>>,
}

impl RecordingExportClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_render_zip_response(
        mut self,
        response: ServiceResponse<ExportRenderZipOutput>,
    ) -> Self {
        self.render_zip_response = Some(response);
        self
    }

    pub fn render_zip_call_count(&self) -> usize {
        self.state.render_zip_requests.count()
    }

    pub fn last_render_zip_request(&self) -> Option<ServiceRequest<ExportRenderZipInput>> {
        self.state.render_zip_requests.last()
    }
}

#[async_trait]
impl ServiceClient for RecordingExportClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        Err(unexpected_call("RecordingExportClient", "health"))
    }
}

#[async_trait]
impl ExportClient for RecordingExportClient {
    async fn render_document(
        &self,
        _request: ServiceRequest<ExportRenderDocumentInput>,
    ) -> CapabilityResult<ServiceResponse<ExportRenderDocumentOutput>> {
        Err(unexpected_call("RecordingExportClient", "render_document"))
    }

    async fn render_json(
        &self,
        _request: ServiceRequest<ExportRenderJsonInput>,
    ) -> CapabilityResult<ServiceResponse<String>> {
        Err(unexpected_call("RecordingExportClient", "render_json"))
    }

    async fn render_zip(
        &self,
        request: ServiceRequest<ExportRenderZipInput>,
    ) -> CapabilityResult<ServiceResponse<ExportRenderZipOutput>> {
        self.state.render_zip_requests.record(request);
        self.render_zip_response
            .clone()
            .ok_or_else(|| unexpected_call("RecordingExportClient", "render_zip"))
    }
}
