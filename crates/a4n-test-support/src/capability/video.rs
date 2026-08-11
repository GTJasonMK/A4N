use std::sync::Arc;

use a4n_capabilities::{
    CapabilityClient, CapabilityResult, JsonRequest, JsonResponse, VideoClient,
};
use a4n_service_protocol::{CapabilityTaskEvent, ServiceHealth};
use async_trait::async_trait;

use super::{
    media::{RequestLedger, send_configured_events},
    unexpected_call,
};

#[derive(Clone, Default)]
pub struct RecordingVideoClient {
    state: Arc<RecordingVideoClientState>,
    generate_events: Option<Vec<CapabilityTaskEvent>>,
    poll_response: Option<JsonResponse>,
    status_response: Option<JsonResponse>,
    delete_response: Option<JsonResponse>,
    preview_response: Option<JsonResponse>,
}

#[derive(Default)]
struct RecordingVideoClientState {
    generate_requests: RequestLedger,
    poll_requests: RequestLedger,
    status_requests: RequestLedger,
    delete_requests: RequestLedger,
    preview_requests: RequestLedger,
}

impl RecordingVideoClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_generate_events(mut self, events: Vec<CapabilityTaskEvent>) -> Self {
        self.generate_events = Some(events);
        self
    }

    pub fn with_poll_response(mut self, response: JsonResponse) -> Self {
        self.poll_response = Some(response);
        self
    }

    pub fn with_status_response(mut self, response: JsonResponse) -> Self {
        self.status_response = Some(response);
        self
    }

    pub fn with_delete_response(mut self, response: JsonResponse) -> Self {
        self.delete_response = Some(response);
        self
    }

    pub fn with_preview_response(mut self, response: JsonResponse) -> Self {
        self.preview_response = Some(response);
        self
    }

    pub fn generate_call_count(&self) -> usize {
        self.state.generate_requests.count()
    }

    pub fn poll_call_count(&self) -> usize {
        self.state.poll_requests.count()
    }

    pub fn status_call_count(&self) -> usize {
        self.state.status_requests.count()
    }

    pub fn delete_call_count(&self) -> usize {
        self.state.delete_requests.count()
    }

    pub fn preview_call_count(&self) -> usize {
        self.state.preview_requests.count()
    }

    pub fn last_generate_request(&self) -> Option<JsonRequest> {
        self.state.generate_requests.last()
    }

    pub fn last_poll_request(&self) -> Option<JsonRequest> {
        self.state.poll_requests.last()
    }

    pub fn last_status_request(&self) -> Option<JsonRequest> {
        self.state.status_requests.last()
    }

    pub fn last_delete_request(&self) -> Option<JsonRequest> {
        self.state.delete_requests.last()
    }

    pub fn last_preview_request(&self) -> Option<JsonRequest> {
        self.state.preview_requests.last()
    }
}

#[async_trait]
impl CapabilityClient for RecordingVideoClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        Err(unexpected_call("RecordingVideoClient", "health"))
    }
}

#[async_trait]
impl VideoClient for RecordingVideoClient {
    async fn generate_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> CapabilityResult<()> {
        self.state.generate_requests.record(request.clone());
        send_configured_events(
            "RecordingVideoClient",
            "generate_events",
            self.generate_events.as_deref(),
            &request.request_id,
            &events,
        )
        .await
    }

    async fn poll(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.state.poll_requests.record(request);
        self.poll_response
            .clone()
            .ok_or_else(|| unexpected_call("RecordingVideoClient", "poll"))
    }

    async fn status(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.state.status_requests.record(request);
        self.status_response
            .clone()
            .ok_or_else(|| unexpected_call("RecordingVideoClient", "status"))
    }

    async fn delete(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.state.delete_requests.record(request);
        self.delete_response
            .clone()
            .ok_or_else(|| unexpected_call("RecordingVideoClient", "delete"))
    }

    async fn preview_request(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.state.preview_requests.record(request);
        self.preview_response
            .clone()
            .ok_or_else(|| unexpected_call("RecordingVideoClient", "preview_request"))
    }

    async fn discover_models(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingVideoClient", "discover_models"))
    }
}
