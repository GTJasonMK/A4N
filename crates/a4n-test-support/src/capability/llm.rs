use std::sync::{Arc, Mutex};

use a4n_llm_client::{JsonRequest, JsonResponse, LlmClient};
use a4n_model_discovery_contract::{ModelDiscoveryRequest, ModelDiscoveryResult};
use a4n_service_client::{ServiceClient, ServiceClientResult as CapabilityResult};
use a4n_service_protocol::{CapabilityTaskEvent, ServiceHealth, ServiceRequest, ServiceResponse};
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

    pub fn probe_requests(&self) -> Vec<JsonRequest> {
        self.state
            .probe_requests
            .lock()
            .expect("recording LLM probe-request lock should not be poisoned")
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
impl ServiceClient for RecordingLlmClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        Err(unexpected_call("RecordingLlmClient", "health"))
    }
}

#[async_trait]
impl LlmClient for RecordingLlmClient {
    async fn stream(
        &self,
        _request: JsonRequest,
        _events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
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

    async fn discover_models(
        &self,
        _request: ServiceRequest<ModelDiscoveryRequest>,
    ) -> CapabilityResult<ServiceResponse<ModelDiscoveryResult>> {
        Err(unexpected_call("RecordingLlmClient", "discover_models"))
    }
}
