mod events;
mod ingest;
mod management;
mod search;

use a4n_service_protocol::{
    HealthStatus, PROTOCOL_VERSION, RAG_INDEX_CAPABILITY_VERSION as RAG_CAPABILITY_VERSION,
    RAG_SERVICE_NAME, ServiceHealth, ServiceVersion,
};
use axum::Json;

pub(crate) use events::ingest_events;
#[cfg(test)]
pub(crate) use events::{completed_ingest_event, failed_ingest_event, progress_ingest_event};
pub(crate) use ingest::ingest;
pub(crate) use management::{
    collection_exists, completeness_report, completeness_report_by_ids, count_records,
    delete_records, drop_collection, health_check, list_collections, metadata_by_filter,
    metadata_by_ids, update_metadata,
};
pub(crate) use search::{query, search, search_text};

pub(crate) async fn healthz() -> Json<ServiceHealth> {
    Json(ServiceHealth {
        service: RAG_SERVICE_NAME.to_string(),
        status: HealthStatus::Healthy,
        version: ServiceVersion {
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: PROTOCOL_VERSION,
            capability_version: RAG_CAPABILITY_VERSION.to_string(),
        },
    })
}
