use a4n_service_protocol::CapabilityTaskEvent;
use async_trait::async_trait;

use super::{CapabilityClient, CapabilityResult, JsonRequest, JsonResponse};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingModelOperation {
    List,
    DownloadInfo,
    Delete,
}

#[async_trait]
pub trait EmbeddingClient: CapabilityClient {
    async fn embed(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn model_operation(
        &self,
        operation: EmbeddingModelOperation,
        request: JsonRequest,
    ) -> CapabilityResult<JsonResponse>;
    async fn download_model_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> CapabilityResult<()>;
}
