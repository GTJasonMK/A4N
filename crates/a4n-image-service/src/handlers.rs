use a4n_service_protocol::{
    HealthStatus, IMAGE_SERVICE_NAME, ServiceHealth, known_service_health_for_host,
};
use axum::{Json, http::HeaderMap};

pub(crate) mod discovery;
pub(crate) mod events;
pub(crate) mod generation;

pub(crate) async fn healthz(headers: HeaderMap) -> Json<ServiceHealth> {
    Json(known_service_health_for_host(
        IMAGE_SERVICE_NAME,
        env!("CARGO_PKG_VERSION"),
        HealthStatus::Healthy,
        a4n_sidecar_bootstrap::host_protocol_version(&headers),
    ))
}
