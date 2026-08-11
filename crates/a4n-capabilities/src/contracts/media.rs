use a4n_service_protocol::CapabilityTaskEvent;
use async_trait::async_trait;

use super::{CapabilityClient, CapabilityResult, JsonRequest, JsonResponse};

#[async_trait]
pub trait ImageClient: CapabilityClient {
    async fn generate_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> CapabilityResult<()>;
    async fn preview_request(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn discover_models(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
}

#[async_trait]
pub trait VideoClient: CapabilityClient {
    async fn generate_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> CapabilityResult<()>;
    async fn poll(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn status(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn delete(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn preview_request(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn discover_models(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
}
