use std::sync::Arc;

use a4n_image_client::{ImageClient, ImageProviderRequest, JsonRequest, JsonResponse};
use a4n_service_client::{ServiceClient, ServiceClientResult as CapabilityResult};
use a4n_service_protocol::{CapabilityTaskEvent, ServiceHealth};
use async_trait::async_trait;

use super::{
    media::{RequestLedger, send_configured_events},
    unexpected_call,
};

#[derive(Clone, Default)]
pub struct RecordingImageClient {
    state: Arc<RecordingImageClientState>,
    generate_events: Option<Vec<CapabilityTaskEvent>>,
    preview_response: Option<JsonResponse>,
}

#[derive(Default)]
struct RecordingImageClientState {
    generate_requests: RequestLedger<ImageProviderRequest>,
    preview_requests: RequestLedger<ImageProviderRequest>,
}

impl RecordingImageClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_generate_events(mut self, events: Vec<CapabilityTaskEvent>) -> Self {
        self.generate_events = Some(events);
        self
    }

    pub fn with_preview_response(mut self, response: JsonResponse) -> Self {
        self.preview_response = Some(response);
        self
    }

    pub fn generate_call_count(&self) -> usize {
        self.state.generate_requests.count()
    }

    pub fn preview_call_count(&self) -> usize {
        self.state.preview_requests.count()
    }

    pub fn last_generate_request(&self) -> Option<ImageProviderRequest> {
        self.state.generate_requests.last()
    }

    pub fn last_preview_request(&self) -> Option<ImageProviderRequest> {
        self.state.preview_requests.last()
    }
}

#[async_trait]
impl ServiceClient for RecordingImageClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        Err(unexpected_call("RecordingImageClient", "health"))
    }
}

#[async_trait]
impl ImageClient for RecordingImageClient {
    async fn generate_events(
        &self,
        request: ImageProviderRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> CapabilityResult<()> {
        self.state.generate_requests.record(request.clone());
        send_configured_events(
            "RecordingImageClient",
            "generate_events",
            self.generate_events.as_deref(),
            &request.request_id,
            &events,
        )
        .await
    }

    async fn preview_request(
        &self,
        request: ImageProviderRequest,
    ) -> CapabilityResult<JsonResponse> {
        self.state.preview_requests.record(request);
        self.preview_response
            .clone()
            .ok_or_else(|| unexpected_call("RecordingImageClient", "preview_request"))
    }

    async fn discover_models(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingImageClient", "discover_models"))
    }
}
