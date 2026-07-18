use a4n_service_protocol::ServiceTaskEvent;
use async_trait::async_trait;

use super::{CapabilityClient, CapabilityResult, JsonRequest, JsonResponse};

#[async_trait]
pub trait LlmClient: CapabilityClient {
    async fn chat(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn stream(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::UnboundedSender<ServiceTaskEvent>,
    ) -> CapabilityResult<()>;
    async fn probe(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn discover_models(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
}
