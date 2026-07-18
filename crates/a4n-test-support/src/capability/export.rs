use std::sync::Arc;

use a4n_capabilities::{
    CapabilityClient, CapabilityResult, ExportClient, JsonRequest, JsonResponse,
};
use a4n_export_core::{
    ExportRenderJsonInput, ExportRenderJsonOutput, ExportRenderZipInput, ExportRenderZipOutput,
};
use a4n_service_protocol::{ServiceHealth, ServiceRequest, ServiceResponse};
use async_trait::async_trait;

use super::{media::RequestLedger, unexpected_call};

#[derive(Clone, Default)]
pub struct RecordingExportClient {
    state: Arc<RecordingExportClientState>,
    render_json_response: Option<JsonResponse>,
    render_zip_response: Option<JsonResponse>,
}

#[derive(Default)]
struct RecordingExportClientState {
    render_json_requests: RequestLedger,
    render_zip_requests: RequestLedger,
}

impl RecordingExportClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_render_json_response(mut self, response: JsonResponse) -> Self {
        self.render_json_response = Some(response);
        self
    }

    pub fn with_render_zip_response(mut self, response: JsonResponse) -> Self {
        self.render_zip_response = Some(response);
        self
    }

    pub fn render_json_call_count(&self) -> usize {
        self.state.render_json_requests.count()
    }

    pub fn render_zip_call_count(&self) -> usize {
        self.state.render_zip_requests.count()
    }

    pub fn last_render_json_request(&self) -> Option<JsonRequest> {
        self.state.render_json_requests.last()
    }

    pub fn last_render_zip_request(&self) -> Option<JsonRequest> {
        self.state.render_zip_requests.last()
    }
}

#[async_trait]
impl CapabilityClient for RecordingExportClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        Err(unexpected_call("RecordingExportClient", "health"))
    }
}

#[async_trait]
impl ExportClient for RecordingExportClient {
    async fn render_json(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.state.render_json_requests.record(request);
        self.render_json_response
            .clone()
            .ok_or_else(|| unexpected_call("RecordingExportClient", "render_json"))
    }

    async fn render_zip(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.state.render_zip_requests.record(request);
        self.render_zip_response
            .clone()
            .ok_or_else(|| unexpected_call("RecordingExportClient", "render_zip"))
    }

    async fn render_json_typed(
        &self,
        _request: ServiceRequest<ExportRenderJsonInput>,
    ) -> CapabilityResult<ServiceResponse<ExportRenderJsonOutput>> {
        Err(unexpected_call(
            "RecordingExportClient",
            "render_json_typed",
        ))
    }

    async fn render_zip_typed(
        &self,
        _request: ServiceRequest<ExportRenderZipInput>,
    ) -> CapabilityResult<ServiceResponse<ExportRenderZipOutput>> {
        Err(unexpected_call("RecordingExportClient", "render_zip_typed"))
    }
}
