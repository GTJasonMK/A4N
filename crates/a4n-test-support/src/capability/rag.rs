use std::sync::Arc;

use a4n_capabilities::{CapabilityClient, CapabilityResult, JsonRequest, JsonResponse, RagClient};
use a4n_service_protocol::{ServiceHealth, ServiceTaskEvent};
use async_trait::async_trait;

use super::{
    media::{RequestLedger, send_configured_events},
    unexpected_call,
};

#[derive(Clone, Default)]
pub struct RecordingRagClient {
    state: Arc<RecordingRagClientState>,
    ingest_events: Option<Vec<ServiceTaskEvent>>,
}

#[derive(Default)]
struct RecordingRagClientState {
    ingest_requests: RequestLedger,
    ingest_event_requests: RequestLedger,
}

impl RecordingRagClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_ingest_events(mut self, events: Vec<ServiceTaskEvent>) -> Self {
        self.ingest_events = Some(events);
        self
    }

    pub fn ingest_call_count(&self) -> usize {
        self.state.ingest_requests.count()
    }

    pub fn ingest_events_call_count(&self) -> usize {
        self.state.ingest_event_requests.count()
    }

    pub fn last_ingest_request(&self) -> Option<JsonRequest> {
        self.state.ingest_requests.last()
    }

    pub fn last_ingest_events_request(&self) -> Option<JsonRequest> {
        self.state.ingest_event_requests.last()
    }
}

#[async_trait]
impl CapabilityClient for RecordingRagClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        Err(unexpected_call("RecordingRagClient", "health"))
    }
}

#[async_trait]
impl RagClient for RecordingRagClient {
    async fn query(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingRagClient", "query"))
    }

    async fn search_text(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingRagClient", "search_text"))
    }

    async fn ingest(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.state.ingest_requests.record(request);
        Err(unexpected_call("RecordingRagClient", "ingest"))
    }

    async fn ingest_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::UnboundedSender<ServiceTaskEvent>,
    ) -> CapabilityResult<()> {
        self.state.ingest_event_requests.record(request.clone());
        send_configured_events(
            "RecordingRagClient",
            "ingest_events",
            self.ingest_events.as_deref(),
            &request.request_id,
            &events,
        )
    }

    async fn search(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingRagClient", "search"))
    }

    async fn delete(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingRagClient", "delete"))
    }

    async fn collection_exists(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingRagClient", "collection_exists"))
    }

    async fn count(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingRagClient", "count"))
    }

    async fn drop_collection(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingRagClient", "drop_collection"))
    }

    async fn list_collections(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingRagClient", "list_collections"))
    }

    async fn update_metadata(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingRagClient", "update_metadata"))
    }

    async fn metadata_by_ids(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingRagClient", "metadata_by_ids"))
    }

    async fn metadata_by_filter(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingRagClient", "metadata_by_filter"))
    }

    async fn health_check(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingRagClient", "health_check"))
    }

    async fn completeness_report(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingRagClient", "completeness_report"))
    }

    async fn completeness_report_by_ids(
        &self,
        _request: JsonRequest,
    ) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call(
            "RecordingRagClient",
            "completeness_report_by_ids",
        ))
    }
}
