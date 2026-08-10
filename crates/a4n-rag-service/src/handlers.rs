mod events;
mod management;
mod search;

use a4n_service_protocol::{
    HealthStatus, RAG_SERVICE_NAME, ServiceHealth, known_service_health_for_host,
};
use axum::{Json, http::HeaderMap};

pub(crate) use events::ingest_events;
#[cfg(test)]
pub(crate) use events::{completed_ingest_event, failed_ingest_event, progress_ingest_event};
pub(crate) use management::{
    completeness_report_by_ids, delete_records, drop_collection, metadata_by_filter,
    metadata_by_ids,
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
