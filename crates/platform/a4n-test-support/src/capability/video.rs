use std::sync::Arc;

use a4n_model_discovery_contract::{ModelDiscoveryRequest, ModelDiscoveryResult};
use a4n_service_client::{ServiceClient, ServiceClientResult as CapabilityResult};
use a4n_service_protocol::{CapabilityTaskEvent, ServiceHealth};
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use a4n_video_client::{
    JsonResponse, VideoClient, VideoDeleteResponse, VideoJobResponse, VideoPollRequest,
    VideoProviderRequest, VideoStatusResponse,
};
use async_trait::async_trait;

use super::{
    media::{RequestLedger, send_configured_events},
    unexpected_call,
};

#[derive(Clone, Default)]
pub struct RecordingVideoClient {
    state: Arc<RecordingVideoClientState>,
    generate_events: Option<Vec<CapabilityTaskEvent>>,
    poll_response: Option<VideoJobResponse>,
    status_response: Option<VideoStatusResponse>,
    delete_response: Option<VideoDeleteResponse>,
    preview_response: Option<JsonResponse>,
}

#[derive(Default)]
struct RecordingVideoClientState {
    generate_requests: RequestLedger<VideoProviderRequest>,
    poll_requests: RequestLedger<VideoPollRequest>,
    status_requests: RequestLedger<VideoPollRequest>,
    delete_requests: RequestLedger<VideoPollRequest>,
    preview_requests: RequestLedger<VideoProviderRequest>,
}

impl RecordingVideoClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_generate_events(mut self, events: Vec<CapabilityTaskEvent>) -> Self {
        self.generate_events = Some(events);
        self
    }

    pub fn with_poll_response(mut self, response: VideoJobResponse) -> Self {
        self.poll_response = Some(response);
        self
    }

    pub fn with_status_response(mut self, response: VideoStatusResponse) -> Self {
        self.status_response = Some(response);
        self
    }

    pub fn with_delete_response(mut self, response: VideoDeleteResponse) -> Self {
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

    pub fn last_generate_request(&self) -> Option<VideoProviderRequest> {
        self.state.generate_requests.last()
    }

    pub fn last_poll_request(&self) -> Option<VideoPollRequest> {
        self.state.poll_requests.last()
    }

    pub fn last_status_request(&self) -> Option<VideoPollRequest> {
        self.state.status_requests.last()
    }

    pub fn last_delete_request(&self) -> Option<VideoPollRequest> {
        self.state.delete_requests.last()
    }

    pub fn last_preview_request(&self) -> Option<VideoProviderRequest> {
        self.state.preview_requests.last()
    }
}

#[async_trait]
impl ServiceClient for RecordingVideoClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        Err(unexpected_call("RecordingVideoClient", "health"))
    }
}

#[async_trait]
impl VideoClient for RecordingVideoClient {
    async fn generate_events(
        &self,
        request: VideoProviderRequest,
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

    async fn poll(&self, request: VideoPollRequest) -> CapabilityResult<VideoJobResponse> {
        self.state.poll_requests.record(request);
        self.poll_response
            .clone()
            .ok_or_else(|| unexpected_call("RecordingVideoClient", "poll"))
    }

    async fn status(&self, request: VideoPollRequest) -> CapabilityResult<VideoStatusResponse> {
        self.state.status_requests.record(request);
        self.status_response
            .clone()
            .ok_or_else(|| unexpected_call("RecordingVideoClient", "status"))
    }

    async fn delete(&self, request: VideoPollRequest) -> CapabilityResult<VideoDeleteResponse> {
        self.state.delete_requests.record(request);
        self.delete_response
            .clone()
            .ok_or_else(|| unexpected_call("RecordingVideoClient", "delete"))
    }

    async fn preview_request(
        &self,
        request: VideoProviderRequest,
    ) -> CapabilityResult<JsonResponse> {
        self.state.preview_requests.record(request);
        self.preview_response
            .clone()
            .ok_or_else(|| unexpected_call("RecordingVideoClient", "preview_request"))
    }

    async fn discover_models(
        &self,
        _request: ServiceRequest<ModelDiscoveryRequest>,
    ) -> CapabilityResult<ServiceResponse<ModelDiscoveryResult>> {
        Err(unexpected_call("RecordingVideoClient", "discover_models"))
    }
}
