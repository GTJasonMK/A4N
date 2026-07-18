use std::sync::Arc;

use a4n_capabilities::{
    CapabilityClient, CapabilityResult, JsonRequest, JsonResponse, VideoClient,
};
use a4n_service_protocol::{ServiceHealth, ServiceTaskEvent};
use async_trait::async_trait;

use super::{
    media::{RequestLedger, send_configured_events},
    unexpected_call,
};

#[derive(Clone, Default)]
pub struct RecordingVideoClient {
    state: Arc<RecordingVideoClientState>,
    generate_events: Option<Vec<ServiceTaskEvent>>,
    poll_response: Option<JsonResponse>,
    preview_response: Option<JsonResponse>,
    probe_response: Option<JsonResponse>,
}

#[derive(Default)]
struct RecordingVideoClientState {
    generate_requests: RequestLedger,
    poll_requests: RequestLedger,
    preview_requests: RequestLedger,
    probe_requests: RequestLedger,
}

impl RecordingVideoClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_generate_events(mut self, events: Vec<ServiceTaskEvent>) -> Self {
        self.generate_events = Some(events);
        self
    }

    pub fn with_poll_response(mut self, response: JsonResponse) -> Self {
        self.poll_response = Some(response);
        self
    }

    pub fn with_preview_response(mut self, response: JsonResponse) -> Self {
        self.preview_response = Some(response);
        self
    }

    pub fn with_probe_response(mut self, response: JsonResponse) -> Self {
        self.probe_response = Some(response);
        self
    }

    pub fn generate_call_count(&self) -> usize {
        self.state.generate_requests.count()
    }

    pub fn poll_call_count(&self) -> usize {
        self.state.poll_requests.count()
    }

    pub fn preview_call_count(&self) -> usize {
        self.state.preview_requests.count()
    }

    pub fn probe_call_count(&self) -> usize {
        self.state.probe_requests.count()
    }

    pub fn last_generate_request(&self) -> Option<JsonRequest> {
        self.state.generate_requests.last()
    }

    pub fn last_poll_request(&self) -> Option<JsonRequest> {
        self.state.poll_requests.last()
    }

    pub fn last_preview_request(&self) -> Option<JsonRequest> {
        self.state.preview_requests.last()
    }

    pub fn last_probe_request(&self) -> Option<JsonRequest> {
        self.state.probe_requests.last()
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
    async fn generate(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.state.generate_requests.record(request);
        Err(unexpected_call("RecordingVideoClient", "generate"))
    }

    async fn generate_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::UnboundedSender<ServiceTaskEvent>,
    ) -> CapabilityResult<()> {
        self.state.generate_requests.record(request.clone());
        send_configured_events(
            "RecordingVideoClient",
            "generate_events",
            self.generate_events.as_deref(),
            &request.request_id,
            &events,
        )
    }

    async fn poll(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.state.poll_requests.record(request);
        self.poll_response
            .clone()
            .ok_or_else(|| unexpected_call("RecordingVideoClient", "poll"))
    }

    async fn preview_request(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.state.preview_requests.record(request);
        self.preview_response
            .clone()
            .ok_or_else(|| unexpected_call("RecordingVideoClient", "preview_request"))
    }

    async fn probe(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.state.probe_requests.record(request);
        self.probe_response
            .clone()
            .ok_or_else(|| unexpected_call("RecordingVideoClient", "probe"))
    }

    async fn discover_models(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingVideoClient", "discover_models"))
    }
}
