mod events;
mod management;
mod search;

use a4n_service_protocol::{
    HealthStatus, RAG_SERVICE_NAME, ServiceHealth, known_service_health,
};
use axum::Json;

pub(crate) use events::ingest_events;
pub(crate) use management::{
    completeness_report_by_ids, delete_records, drop_collection, metadata_by_filter,
    metadata_by_ids,
};
pub(crate) use search::{query, search, search_text};

pub(crate) async fn healthz() -> Json<ServiceHealth> {
    Json(known_service_health(
        RAG_SERVICE_NAME,
        env!("CARGO_PKG_VERSION"),
        HealthStatus::Healthy,
    ))
}
