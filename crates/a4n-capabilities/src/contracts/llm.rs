use a4n_service_protocol::CapabilityTaskEvent;
use async_trait::async_trait;

use super::{CapabilityClient, CapabilityResult, JsonRequest, JsonResponse};

#[async_trait]
pub trait LlmClient: CapabilityClient {
    async fn stream(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> CapabilityResult<()>;
    async fn probe(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn discover_models(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
}
