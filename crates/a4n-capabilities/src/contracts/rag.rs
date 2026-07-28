use a4n_service_protocol::CapabilityTaskEvent;
use async_trait::async_trait;

use super::{CapabilityClient, CapabilityResult, JsonRequest, JsonResponse};

#[async_trait]
pub trait RagClient: CapabilityClient {
    async fn query(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn search_text(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn ingest(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn ingest_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> CapabilityResult<()>;
    async fn search(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn delete(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn collection_exists(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn count(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn drop_collection(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn list_collections(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn update_metadata(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn metadata_by_ids(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn metadata_by_filter(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn health_check(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn completeness_report(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn completeness_report_by_ids(
        &self,
        request: JsonRequest,
    ) -> CapabilityResult<JsonResponse>;
}
