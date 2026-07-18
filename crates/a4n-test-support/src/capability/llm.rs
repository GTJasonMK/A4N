use std::sync::{Arc, Mutex};

use a4n_capabilities::{CapabilityClient, CapabilityResult, JsonRequest, JsonResponse, LlmClient};
use a4n_service_protocol::{ServiceHealth, ServiceTaskEvent};
use async_trait::async_trait;

use super::unexpected_call;

#[derive(Clone)]
pub struct RecordingLlmClient {
    state: Arc<RecordingLlmClientState>,
    probe_response: JsonResponse,
}

#[derive(Default)]
struct RecordingLlmClientState {
    probe_requests: Mutex<Vec<JsonRequest>>,
    chat_requests: Mutex<Vec<JsonRequest>>,
}

impl RecordingLlmClient {
    pub fn new(probe_response: JsonResponse) -> Self {
        Self {
            state: Arc::new(RecordingLlmClientState::default()),
            probe_response,
        }
    }

    pub fn probe_call_count(&self) -> usize {
        self.state
            .probe_requests
            .lock()
            .expect("recording LLM probe-request lock should not be poisoned")
            .len()
    }

    pub fn chat_call_count(&self) -> usize {
        self.state
            .chat_requests
            .lock()
            .expect("recording LLM chat-request lock should not be poisoned")
            .len()
    }

    pub fn probe_requests(&self) -> Vec<JsonRequest> {
        self.state
            .probe_requests
            .lock()
            .expect("recording LLM probe-request lock should not be poisoned")
            .clone()
    }

    pub fn chat_requests(&self) -> Vec<JsonRequest> {
        self.state
            .chat_requests
            .lock()
            .expect("recording LLM chat-request lock should not be poisoned")
            .clone()
    }

    pub fn last_probe_request(&self) -> Option<JsonRequest> {
        self.state
            .probe_requests
            .lock()
            .expect("recording LLM probe-request lock should not be poisoned")
            .last()
            .cloned()
    }
}

#[async_trait]
impl CapabilityClient for RecordingLlmClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        Err(unexpected_call("RecordingLlmClient", "health"))
    }
}

#[async_trait]
impl LlmClient for RecordingLlmClient {
    async fn chat(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.state
            .chat_requests
            .lock()
            .expect("recording LLM chat-request lock should not be poisoned")
            .push(request);
        Err(unexpected_call("RecordingLlmClient", "chat"))
    }

    async fn stream(
        &self,
        _request: JsonRequest,
        _events: tokio::sync::mpsc::UnboundedSender<ServiceTaskEvent>,
    ) -> CapabilityResult<()> {
        Err(unexpected_call("RecordingLlmClient", "stream"))
    }

    async fn probe(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.state
            .probe_requests
            .lock()
            .expect("recording LLM probe-request lock should not be poisoned")
            .push(request);
        Ok(self.probe_response.clone())
    }

    async fn discover_models(&self, _request: JsonRequest) -> CapabilityResult<JsonResponse> {
        Err(unexpected_call("RecordingLlmClient", "discover_models"))
    }
}
