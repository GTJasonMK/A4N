use super::contracts::{CapabilityClient, CapabilityResult, JsonRequest, JsonResponse, RagClient};
use super::transport::HttpSidecarTransport;
use a4n_service_protocol::{RAG_SERVICE_NAME, ServiceHealth, ServiceTaskEvent};
use async_trait::async_trait;

pub struct HttpRagClient {
    pub(super) transport: HttpSidecarTransport,
}

impl HttpRagClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url).with_expected_service(RAG_SERVICE_NAME),
        }
    }
}

#[async_trait]
impl CapabilityClient for HttpRagClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
impl RagClient for HttpRagClient {
    async fn query(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/rag/query", request).await
    }

    async fn search_text(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/rag/search_text", request).await
    }

    async fn ingest(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/rag/ingest", request).await
    }

    async fn ingest_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::UnboundedSender<ServiceTaskEvent>,
    ) -> CapabilityResult<()> {
        self.transport
            .post_event_stream("/rag/ingest/events", request, events)
            .await
    }

    async fn search(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/rag/search", request).await
    }

    async fn delete(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/rag/delete", request).await
    }

    async fn collection_exists(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/rag/collection_exists", request)
            .await
    }

    async fn count(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/rag/count", request).await
    }

    async fn drop_collection(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/rag/drop_collection", request)
            .await
    }

    async fn list_collections(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/rag/list_collections", request)
            .await
    }

    async fn update_metadata(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/rag/update_metadata", request)
            .await
    }

    async fn metadata_by_ids(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/rag/metadata_by_ids", request)
            .await
    }

    async fn metadata_by_filter(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/rag/metadata_by_filter", request)
            .await
    }

    async fn health_check(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/rag/health_check", request).await
    }

    async fn completeness_report(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/rag/completeness_report", request)
            .await
    }

    async fn completeness_report_by_ids(
        &self,
        request: JsonRequest,
    ) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/rag/completeness_report_by_ids", request)
            .await
    }
}
