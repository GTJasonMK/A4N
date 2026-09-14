use a4n_error::AppError;
use a4n_rag_client::{
    CompletenessByIdsResponse, DeleteResponse, DropCollectionResponse, MetadataByFilterResponse,
    MetadataByIdsResponse,
};
use a4n_rag_core::vector_store::{CompletenessReport, DeleteFilter, EmbeddingRecord};
use a4n_rag_sidecar_contract::{
    CompletenessByIdsPayload, DeleteFilterPayload, DeleteRecordsPayload, DropCollectionPayload,
    MetadataByFilterPayload, MetadataByIdsPayload,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;

use super::SidecarVectorStore;
use crate::sidecar_clients::rag::events::handle_rag_ingest_event;
use crate::sidecar_clients::rag::request::{rag_service_request, rag_service_request_typed};
use crate::sidecar_clients::{
    response_payload, service_app_error, service_error_app_error, typed_response_payload,
};

impl SidecarVectorStore {
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

    pub(super) async fn call<T: DeserializeOwned>(
        &self,
        operation: RagOperation,
        payload: impl Serialize,
    ) -> Result<T, AppError> {
        let request =
            rag_service_request(operation.trace_span(), payload).map_err(service_app_error)?;
        let response = match operation {
            RagOperation::Query => self.client.query(request).await,
            RagOperation::SearchText => self.client.search_text(request).await,
            RagOperation::Search => self.client.search(request).await,
        }
        .map_err(service_error_app_error)?;
        response_payload(response, "rag").map_err(service_app_error)
    }
}

pub(super) enum RagOperation {
    Query,
    SearchText,
    Search,
}

impl RagOperation {
    fn trace_span(&self) -> &'static str {
        match self {
            Self::Query => "rag.query",
            Self::SearchText => "rag.search_text",
            Self::Search => "rag.search",
        }
    }
}

fn delete_filter_payload(filter: DeleteFilter) -> DeleteFilterPayload {
    DeleteFilterPayload {
        ids: filter.ids,
        id_prefixes: filter.id_prefixes,
    }
}
