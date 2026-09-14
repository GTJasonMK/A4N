//! Typed client for the RAG sidecar service.

use a4n_rag_sidecar_contract::{
    CompletenessByIdsPayload, CompletenessReport, DeleteRecordsPayload, DropCollectionPayload,
    MetadataByFilterPayload, MetadataByIdsPayload,
};
use a4n_service_client::{
    HttpClientFactory, HttpSidecarTransport, ServiceClient, ServiceClientConfig,
    ServiceClientResult,
};
use a4n_service_protocol::{
    CapabilityTaskEvent, RAG_COMPLETENESS_REPORT_BY_IDS_PATH, RAG_DELETE_PATH,
    RAG_DROP_COLLECTION_PATH, RAG_INGEST_EVENTS_PATH, RAG_METADATA_BY_FILTER_PATH,
    RAG_METADATA_BY_IDS_PATH, RAG_QUERY_PATH, RAG_SEARCH_PATH, RAG_SEARCH_TEXT_PATH,
    RAG_SERVICE_NAME, ServiceHealth, ServiceRequest, ServiceResponse,
};
use async_trait::async_trait;
use std::collections::HashMap;
#[cfg(any(test, feature = "test-support"))]
use std::time::Duration;

pub type JsonRequest = ServiceRequest<serde_json::Value>;
pub type JsonResponse = ServiceResponse<serde_json::Value>;
pub type MetadataByIdsRequest = ServiceRequest<MetadataByIdsPayload>;
pub type MetadataByIdsResponse = ServiceResponse<HashMap<String, serde_json::Value>>;
pub type CompletenessByIdsRequest = ServiceRequest<CompletenessByIdsPayload>;
pub type CompletenessByIdsResponse = ServiceResponse<CompletenessReport>;
pub type DropCollectionRequest = ServiceRequest<DropCollectionPayload>;
pub type DropCollectionResponse = ServiceResponse<()>;
pub type DeleteRequest = ServiceRequest<DeleteRecordsPayload>;
pub type DeleteResponse = ServiceResponse<usize>;
pub type MetadataByFilterRequest = ServiceRequest<MetadataByFilterPayload>;
pub type MetadataByFilterResponse = ServiceResponse<HashMap<String, serde_json::Value>>;

pub struct HttpRagClient {
    transport: HttpSidecarTransport,
}

impl HttpRagClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(
                base_url,
                ServiceClientConfig::for_service(RAG_SERVICE_NAME),
            ),
        }
    }

    pub fn new_with_auth_token(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(
                base_url,
                ServiceClientConfig::for_service(RAG_SERVICE_NAME),
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
impl ServiceClient for HttpRagClient {
    async fn health(&self) -> ServiceClientResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
pub trait RagClient: ServiceClient {
    async fn query(&self, request: JsonRequest) -> ServiceClientResult<JsonResponse>;
    async fn search_text(&self, request: JsonRequest) -> ServiceClientResult<JsonResponse>;
    async fn ingest_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> ServiceClientResult<()>;
    async fn search(&self, request: JsonRequest) -> ServiceClientResult<JsonResponse>;
    async fn delete(&self, request: DeleteRequest) -> ServiceClientResult<DeleteResponse>;
    async fn drop_collection(
        &self,
        request: DropCollectionRequest,
    ) -> ServiceClientResult<DropCollectionResponse>;
    async fn metadata_by_ids(
        &self,
        request: MetadataByIdsRequest,
    ) -> ServiceClientResult<MetadataByIdsResponse>;
    async fn metadata_by_filter(
        &self,
        request: MetadataByFilterRequest,
    ) -> ServiceClientResult<MetadataByFilterResponse>;
    async fn completeness_report_by_ids(
        &self,
        request: CompletenessByIdsRequest,
    ) -> ServiceClientResult<CompletenessByIdsResponse>;
}

#[async_trait]
impl RagClient for HttpRagClient {
    async fn query(&self, request: JsonRequest) -> ServiceClientResult<JsonResponse> {
        self.transport.post(RAG_QUERY_PATH, request).await
    }

    async fn search_text(&self, request: JsonRequest) -> ServiceClientResult<JsonResponse> {
        self.transport.post(RAG_SEARCH_TEXT_PATH, request).await
    }

    async fn ingest_events(
        &self,
        request: JsonRequest,
        events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
    ) -> ServiceClientResult<()> {
        self.transport
            .post_event_stream(RAG_INGEST_EVENTS_PATH, request, events)
            .await
    }

    async fn search(&self, request: JsonRequest) -> ServiceClientResult<JsonResponse> {
        self.transport.post(RAG_SEARCH_PATH, request).await
    }

    async fn delete(&self, request: DeleteRequest) -> ServiceClientResult<DeleteResponse> {
        self.transport.post(RAG_DELETE_PATH, request).await
    }

    async fn drop_collection(
        &self,
        request: DropCollectionRequest,
    ) -> ServiceClientResult<DropCollectionResponse> {
        self.transport.post(RAG_DROP_COLLECTION_PATH, request).await
    }

    async fn metadata_by_ids(
        &self,
        request: MetadataByIdsRequest,
    ) -> ServiceClientResult<MetadataByIdsResponse> {
        self.transport.post(RAG_METADATA_BY_IDS_PATH, request).await
    }

    async fn metadata_by_filter(
        &self,
        request: MetadataByFilterRequest,
    ) -> ServiceClientResult<MetadataByFilterResponse> {
        self.transport
            .post(RAG_METADATA_BY_FILTER_PATH, request)
            .await
    }

    async fn completeness_report_by_ids(
        &self,
        request: CompletenessByIdsRequest,
    ) -> ServiceClientResult<CompletenessByIdsResponse> {
        self.transport
            .post(RAG_COMPLETENESS_REPORT_BY_IDS_PATH, request)
            .await
    }
}
