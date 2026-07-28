use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

use a4n_capabilities::{
    CapabilityClient, CapabilityResult, EmbeddingClient, JsonRequest, JsonResponse,
};
use a4n_service_protocol::{CapabilityTaskEvent, ServiceHealth};
use async_trait::async_trait;

use super::{media::RequestLedger, unexpected_call};

#[derive(Clone, Default)]
pub struct PendingEmbeddingDownloadClient {
    state: Arc<PendingEmbeddingDownloadState>,
}

#[derive(Clone)]
pub struct PendingEmbeddingDownloadObserver {
    state: Arc<PendingEmbeddingDownloadState>,
}

#[derive(Default)]
struct PendingEmbeddingDownloadState {
    requests: RequestLedger,
    stream_started: AtomicBool,
    stream_dropped: AtomicBool,
    stream_started_notify: tokio::sync::Notify,
}

struct StreamDropSignal(Arc<PendingEmbeddingDownloadState>);

impl Drop for StreamDropSignal {
    fn drop(&mut self) {
        self.0.stream_dropped.store(true, Ordering::Release);
    }
}

impl PendingEmbeddingDownloadClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observer(&self) -> PendingEmbeddingDownloadObserver {
        PendingEmbeddingDownloadObserver {
            state: Arc::clone(&self.state),
        }
    }

    pub fn download_model_events_call_count(&self) -> usize {
        self.state.requests.count()
    }

    pub fn last_download_model_events_request(&self) -> Option<JsonRequest> {
        self.state.requests.last()
    }
}

impl PendingEmbeddingDownloadObserver {
    pub async fn wait_until_started(&self) {
        while !self.state.stream_started.load(Ordering::Acquire) {
            self.state.stream_started_notify.notified().await;
        }
    }

    pub fn stream_was_dropped(&self) -> bool {
        self.state.stream_dropped.load(Ordering::Acquire)
    }
}

#[async_trait]
impl CapabilityClient for PendingEmbeddingDownloadClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        Err(unexpected_call("PendingEmbeddingDownloadClient", "health"))
    }
}

#[async_trait]
impl EmbeddingClient for PendingEmbeddingDownloadClient {
    async fn embed(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("PendingEmbeddingDownloadClient", "embed"))
    }

    async fn model_operation(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call(
            "PendingEmbeddingDownloadClient",
            "model_operation",
        ))
    }

    async fn download_model_events(
        &self,
        request: JsonRequest,
        _events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> CapabilityResult<()> {
        self.state.requests.record(request);
        let _drop_signal = StreamDropSignal(Arc::clone(&self.state));
        self.state.stream_started.store(true, Ordering::Release);
        self.state.stream_started_notify.notify_one();
        std::future::pending().await
    }
}

#[derive(Clone)]
pub struct RecordingEmbeddingClient {
    state: Arc<RecordingEmbeddingClientState>,
    embed_response: JsonResponse,
}

#[derive(Default)]
struct RecordingEmbeddingClientState {
    embed_requests: Mutex<Vec<JsonRequest>>,
}

impl RecordingEmbeddingClient {
    pub fn new(embed_response: JsonResponse) -> Self {
        Self {
            state: Arc::new(RecordingEmbeddingClientState::default()),
            embed_response,
        }
    }

    pub fn embed_call_count(&self) -> usize {
        self.state
            .embed_requests
            .lock()
            .expect("recording embedding request lock should not be poisoned")
            .len()
    }

    pub fn embed_requests(&self) -> Vec<JsonRequest> {
        self.state
            .embed_requests
            .lock()
            .expect("recording embedding request lock should not be poisoned")
            .clone()
    }

    pub fn last_embed_request(&self) -> Option<JsonRequest> {
        self.state
            .embed_requests
            .lock()
            .expect("recording embedding request lock should not be poisoned")
            .last()
            .cloned()
    }
}

#[async_trait]
impl CapabilityClient for RecordingEmbeddingClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        Err(unexpected_call("RecordingEmbeddingClient", "health"))
    }
}

#[async_trait]
impl EmbeddingClient for RecordingEmbeddingClient {
    async fn embed(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.state
            .embed_requests
            .lock()
            .expect("recording embedding request lock should not be poisoned")
            .push(request);
        Ok(self.embed_response.clone())
    }

    async fn model_operation(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call(
            "RecordingEmbeddingClient",
            "model_operation",
        ))
    }

    async fn download_model_events(
        &self,
        _request: JsonRequest,
        _events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> CapabilityResult<()> {
        Err(unexpected_call(
            "RecordingEmbeddingClient",
            "download_model_events",
        ))
    }
}
