use super::contracts::{CapabilityClient, CapabilityResult, JsonRequest, JsonResponse, LlmClient};
use super::transport::HttpSidecarTransport;
use a4n_service_protocol::{LLM_SERVICE_NAME, ServiceHealth, ServiceTaskEvent};
use async_trait::async_trait;

pub struct HttpLlmClient {
    pub(super) transport: HttpSidecarTransport,
}

impl HttpLlmClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url).with_expected_service(LLM_SERVICE_NAME),
        }
    }
}

#[async_trait]
impl CapabilityClient for HttpLlmClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
impl LlmClient for HttpLlmClient {
    async fn chat(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/llm/chat", request).await
    }

    async fn stream(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::UnboundedSender<ServiceTaskEvent>,
    ) -> CapabilityResult<()> {
        self.transport
            .post_event_stream("/llm/stream", request, events)
            .await
    }

    async fn probe(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/llm/probe", request).await
    }

    async fn discover_models(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/llm/models", request).await
    }
}
