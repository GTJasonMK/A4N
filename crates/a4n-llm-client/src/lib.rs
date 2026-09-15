//! Typed client for the LLM sidecar service.

use a4n_llm_sidecar_contract::{LlmChatRequest, LlmGatewayPayload, LlmProbeOutput};
use a4n_model_discovery_contract::{ModelDiscoveryRequest, ModelDiscoveryResult};
use a4n_service_client::{
    HttpClientFactory, HttpSidecarTransport, ServiceClient, ServiceClientConfig,
    ServiceClientResult,
};
use a4n_service_protocol::{
    CapabilityTaskEvent, LLM_SERVICE_NAME, ServiceHealth, ServiceRequest, ServiceResponse,
};
use async_trait::async_trait;
#[cfg(any(test, feature = "test-support"))]
use std::time::Duration;

pub type LlmGatewayRequest = ServiceRequest<LlmGatewayPayload<LlmChatRequest>>;
pub type LlmProbeResponse = ServiceResponse<LlmProbeOutput>;

pub struct HttpLlmClient {
    transport: HttpSidecarTransport,
}

impl HttpLlmClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(
                base_url,
                ServiceClientConfig::for_service(LLM_SERVICE_NAME),
            ),
        }
    }

    pub fn new_with_auth_token(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(
                base_url,
                ServiceClientConfig::for_service(LLM_SERVICE_NAME),
            )
            .with_auth_token(token),
        }
    }

    pub fn with_http_client_factory(mut self, factory: HttpClientFactory) -> Self {
        self.transport = self.transport.with_http_client_factory(factory);
        self
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn with_test_timeouts(
        mut self,
        connect_timeout: Duration,
        request_timeout: Duration,
        stream_idle_timeout: Duration,
        health_timeout: Duration,
    ) -> Self {
        self.transport = self.transport.with_test_timeouts(
            connect_timeout,
            request_timeout,
            stream_idle_timeout,
            health_timeout,
        );
        self
    }
}

#[async_trait]
impl ServiceClient for HttpLlmClient {
    async fn health(&self) -> ServiceClientResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
pub trait LlmClient: ServiceClient {
    async fn stream(
        &self,
        request: LlmGatewayRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> ServiceClientResult<()>;
    async fn probe(&self, request: LlmGatewayRequest) -> ServiceClientResult<LlmProbeResponse>;
    async fn discover_models(
        &self,
        request: ServiceRequest<ModelDiscoveryRequest>,
    ) -> ServiceClientResult<ServiceResponse<ModelDiscoveryResult>>;
}

#[async_trait]
impl LlmClient for HttpLlmClient {
    async fn stream(
        &self,
        request: LlmGatewayRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> ServiceClientResult<()> {
        self.transport
            .post_event_stream("/llm/stream", request, events)
            .await
    }

    async fn probe(&self, request: LlmGatewayRequest) -> ServiceClientResult<LlmProbeResponse> {
        self.transport.post("/llm/probe", request).await
    }

    async fn discover_models(
        &self,
        request: ServiceRequest<ModelDiscoveryRequest>,
    ) -> ServiceClientResult<ServiceResponse<ModelDiscoveryResult>> {
        self.transport.post("/llm/models", request).await
    }
}
