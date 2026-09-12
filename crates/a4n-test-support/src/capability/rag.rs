use std::sync::Arc;

use a4n_rag_client::{JsonRequest, JsonResponse, RagClient};
use a4n_service_client::{ServiceClient, ServiceClientResult as CapabilityResult};
use a4n_service_protocol::{CapabilityTaskEvent, ServiceHealth};
use async_trait::async_trait;

use super::{
    media::{RequestLedger, send_configured_events},
    unexpected_call,
};

#[derive(Clone, Default)]
pub struct RecordingRagClient {
    state: Arc<RecordingRagClientState>,
    ingest_events: Option<Vec<CapabilityTaskEvent>>,
}

#[derive(Default)]
struct RecordingRagClientState {
    ingest_event_requests: RequestLedger,
}

impl RecordingRagClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_ingest_events(mut self, events: Vec<CapabilityTaskEvent>) -> Self {
        self.ingest_events = Some(events);
        self
    }

    pub fn ingest_events_call_count(&self) -> usize {
        self.state.ingest_event_requests.count()
    }

    pub fn last_ingest_events_request(&self) -> Option<JsonRequest> {
        self.state.ingest_event_requests.last()
    }
}

#[async_trait]
impl ServiceClient for RecordingRagClient {
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

    async fn ingest_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> CapabilityResult<()> {
        self.state.ingest_event_requests.record(request.clone());
        send_configured_events(
            "RecordingRagClient",
            "ingest_events",
            self.ingest_events.as_deref(),
            &request.request_id,
            &events,
        )
        .await
    }

    async fn search(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingRagClient", "search"))
    }

    async fn delete(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingRagClient", "delete"))
    }

    async fn drop_collection(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingRagClient", "drop_collection"))
    }

    async fn metadata_by_ids(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingRagClient", "metadata_by_ids"))
    }

    async fn metadata_by_filter(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingRagClient", "metadata_by_filter"))
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
