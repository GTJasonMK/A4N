mod events;
mod ingest;
mod management;
mod search;

use a4n_service_protocol::{
    HealthStatus, RAG_SERVICE_NAME, ServiceHealth, known_service_health_for_host,
};
use axum::{Json, http::HeaderMap};

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

pub(crate) async fn healthz(headers: HeaderMap) -> Json<ServiceHealth> {
    Json(known_service_health_for_host(
        RAG_SERVICE_NAME,
        env!("CARGO_PKG_VERSION"),
        HealthStatus::Healthy,
        a4n_sidecar_bootstrap::host_protocol_version(&headers),
    ))
}
