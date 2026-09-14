use a4n_error::AppError;
use a4n_rag_client::{
    CompletenessByIdsResponse, DeleteResponse, DropCollectionResponse, MetadataByFilterResponse,
    MetadataByIdsResponse, QueryResponse, SearchResponse, SearchTextResponse,
};
use a4n_rag_core::vector_store::{
    CompletenessReport, DeleteFilter, EmbeddingRecord, SearchFilter, SearchResult,
};
use a4n_rag_sidecar_contract::{
    CompletenessByIdsPayload, DeleteFilterPayload, DeleteRecordsPayload, DropCollectionPayload,
    MetadataByFilterPayload, MetadataByIdsPayload, QueryPayload, SearchFilterPayload,
    SearchPayload, SearchResultPayload, SearchTextPayload,
};
use serde_json::json;

use super::SidecarVectorStore;
use crate::sidecar_clients::rag::events::handle_rag_ingest_event;
use crate::sidecar_clients::rag::request::{rag_service_request, rag_service_request_typed};
use crate::sidecar_clients::{service_app_error, service_error_app_error, typed_response_payload};

impl SidecarVectorStore {
    pub(super) async fn query_typed(
        &self,
        collection: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        let request = rag_service_request_typed(
            "rag.query",
            QueryPayload {
                collection: collection.to_string(),
                query_embedding,
                top_k,
                filters: filters.map(search_filter_payload),
            },
        )
        .map_err(service_app_error)?;
        let response: QueryResponse = self
            .client
            .query(request)
            .await
            .map_err(service_error_app_error)?;
        let results = typed_response_payload(response, "rag").map_err(service_app_error)?;
        Ok(results.into_iter().map(core_search_result).collect())
    }

    pub(super) async fn search_text_typed(
        &self,
        collection: &str,
        query_text: &str,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        let request = rag_service_request_typed(
            "rag.search_text",
            SearchTextPayload {
                collection: collection.to_string(),
                query_text: query_text.to_string(),
                top_k,
                filters: filters.map(search_filter_payload),
            },
        )
        .map_err(service_app_error)?;
        let response: SearchTextResponse = self
            .client
            .search_text(request)
            .await
            .map_err(service_error_app_error)?;
        let results = typed_response_payload(response, "rag").map_err(service_app_error)?;
        Ok(results.into_iter().map(core_search_result).collect())
    }

    pub(super) async fn hybrid_search_typed(
        &self,
        collection: &str,
        query_text: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        let request = rag_service_request_typed(
            "rag.search",
            SearchPayload {
                collection: collection.to_string(),
                query_text: query_text.to_string(),
                query_embedding,
                top_k,
                filters: filters.map(search_filter_payload),
            },
        )
        .map_err(service_app_error)?;
        let response: SearchResponse = self
            .client
            .search(request)
            .await
            .map_err(service_error_app_error)?;
        let results = typed_response_payload(response, "rag").map_err(service_app_error)?;
        Ok(results.into_iter().map(core_search_result).collect())
    }

    pub(super) async fn delete_typed(
        &self,
        collection: &str,
        filter: DeleteFilter,
    ) -> Result<usize, AppError> {
        let request = rag_service_request_typed(
            "rag.delete",
            DeleteRecordsPayload {
                collection: collection.to_string(),
                filter: delete_filter_payload(filter),
            },
        )
        .map_err(service_app_error)?;
        let response: DeleteResponse = self
            .client
            .delete(request)
            .await
            .map_err(service_error_app_error)?;
        typed_response_payload(response, "rag").map_err(service_app_error)
    }

    pub(super) async fn drop_collection_typed(&self, collection: &str) -> Result<(), AppError> {
        let request = rag_service_request_typed(
            "rag.drop_collection",
            DropCollectionPayload {
                collection: collection.to_string(),
            },
        )
        .map_err(service_app_error)?;
        let response: DropCollectionResponse = self
            .client
            .drop_collection(request)
            .await
            .map_err(service_error_app_error)?;
        typed_response_payload(response, "rag").map_err(service_app_error)
    }

    pub(super) async fn completeness_report_by_ids_typed(
        &self,
        collection: &str,
        expected_ids: &[String],
    ) -> Result<CompletenessReport, AppError> {
        let request = rag_service_request_typed(
            "rag.completeness_report_by_ids",
            CompletenessByIdsPayload {
                collection: collection.to_string(),
                expected_ids: expected_ids.to_vec(),
            },
        )
        .map_err(service_app_error)?;
        let response: CompletenessByIdsResponse = self
            .client
            .completeness_report_by_ids(request)
            .await
            .map_err(service_error_app_error)?;
        let report = typed_response_payload(response, "rag").map_err(service_app_error)?;
        Ok(CompletenessReport {
            collection: report.collection,
            expected: report.expected,
            actual_vector: report.actual_vector,
            actual_bm25: report.actual_bm25,
            is_complete: report.is_complete,
            missing_vector_ids: report.missing_vector_ids,
            missing_bm25_ids: report.missing_bm25_ids,
            missing_ids: report.missing_ids,
        })
    }

    pub(super) async fn metadata_by_ids_typed(
        &self,
        collection: &str,
        ids: &[String],
    ) -> Result<std::collections::HashMap<String, serde_json::Value>, AppError> {
        let request = rag_service_request_typed(
            "rag.metadata_by_ids",
            MetadataByIdsPayload {
                collection: collection.to_string(),
                ids: ids.to_vec(),
            },
        )
        .map_err(service_app_error)?;
        let response: MetadataByIdsResponse = self
            .client
            .metadata_by_ids(request)
            .await
            .map_err(service_error_app_error)?;
        typed_response_payload(response, "rag").map_err(service_app_error)
    }

    pub(super) async fn metadata_by_filter_typed(
        &self,
        collection: &str,
        filter: DeleteFilter,
    ) -> Result<std::collections::HashMap<String, serde_json::Value>, AppError> {
        let request = rag_service_request_typed(
            "rag.metadata_by_filter",
            MetadataByFilterPayload {
                collection: collection.to_string(),
                filter: delete_filter_payload(filter),
            },
        )
        .map_err(service_app_error)?;
        let response: MetadataByFilterResponse = self
            .client
            .metadata_by_filter(request)
            .await
            .map_err(service_error_app_error)?;
        typed_response_payload(response, "rag").map_err(service_app_error)
    }

    pub(super) async fn ingest(
        &self,
        collection: &str,
        records: Vec<EmbeddingRecord>,
    ) -> Result<usize, AppError> {
        let request = rag_service_request(
            "rag.ingest",
            json!({
                "collection": collection,
                "records": records
            }),
        )
        .map_err(service_app_error)?;
        let (event_tx, mut event_rx) =
            tokio::sync::mpsc::channel(a4n_service_protocol::SIDECAR_EVENT_QUEUE_CAPACITY);
        let mut request_future = Box::pin(self.client.ingest_events(request, event_tx));
        let mut transport_complete = false;
        loop {
            tokio::select! {
                result = &mut request_future, if !transport_complete => {
                    result.map_err(service_error_app_error)?;
                    transport_complete = true;
                }
                event = event_rx.recv() => {
                    let event = event.ok_or_else(|| {
                        service_app_error("rag sidecar ingest 事件流结束但缺少终止事件")
                    })?;
                    if let Some(written) = handle_rag_ingest_event(event)? {
                        return Ok(written);
                    }
                }
            }
        }
    }
}

fn delete_filter_payload(filter: DeleteFilter) -> DeleteFilterPayload {
    DeleteFilterPayload {
        ids: filter.ids,
        id_prefixes: filter.id_prefixes,
    }
}

fn search_filter_payload(filter: SearchFilter) -> SearchFilterPayload {
    SearchFilterPayload {
        equals: filter.equals,
        chapter_range: filter.chapter_range,
        data_types: filter.data_types,
    }
}

fn core_search_result(result: SearchResultPayload) -> SearchResult {
    SearchResult {
        id: result.id,
        content: result.content,
        score: result.score,
        metadata: result.metadata,
    }
}
