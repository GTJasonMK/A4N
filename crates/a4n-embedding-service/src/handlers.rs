use a4n_service_protocol::{
    EMBEDDING_SERVICE_NAME, HealthStatus, ServiceHealth, known_service_health_for_host,
};
use axum::{Json, http::HeaderMap};

pub(crate) mod embedding;
#[cfg(feature = "fastembed")]
pub(crate) mod events;
#[cfg(feature = "fastembed")]
pub(crate) mod models;

pub(crate) async fn healthz(headers: HeaderMap) -> Json<ServiceHealth> {
    Json(known_service_health_for_host(
        EMBEDDING_SERVICE_NAME,
        env!("CARGO_PKG_VERSION"),
        HealthStatus::Healthy,
        a4n_sidecar_bootstrap::host_protocol_version(&headers),
    ))
}
