use a4n_core::error::AppError;
use a4n_rag_core::vector_store::EmbeddingRecord;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;

use super::SidecarVectorStore;
use crate::sidecar_clients::rag::events::handle_rag_ingest_event;
use crate::sidecar_clients::rag::request::rag_service_request;
use crate::sidecar_clients::{response_payload, service_app_error, service_error_app_error};

impl SidecarVectorStore {
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
            RagOperation::Delete => self.client.delete(request).await,
            RagOperation::DropCollection => self.client.drop_collection(request).await,
            RagOperation::MetadataByIds => self.client.metadata_by_ids(request).await,
            RagOperation::MetadataByFilter => self.client.metadata_by_filter(request).await,
            RagOperation::CompletenessReportByIds => {
                self.client.completeness_report_by_ids(request).await
            }
        }
        .map_err(service_error_app_error)?;
        response_payload(response, "rag").map_err(service_app_error)
    }
}

pub(super) enum RagOperation {
    Query,
    SearchText,
    Search,
    Delete,
    DropCollection,
    MetadataByIds,
    MetadataByFilter,
    CompletenessReportByIds,
}

impl RagOperation {
    fn trace_span(&self) -> &'static str {
        match self {
            Self::Query => "rag.query",
            Self::SearchText => "rag.search_text",
            Self::Search => "rag.search",
            Self::Delete => "rag.delete",
            Self::DropCollection => "rag.drop_collection",
            Self::MetadataByIds => "rag.metadata_by_ids",
            Self::MetadataByFilter => "rag.metadata_by_filter",
            Self::CompletenessReportByIds => "rag.completeness_report_by_ids",
        }
    }
}
