use a4n_service_protocol::{
    EMBEDDING_SERVICE_NAME, HealthStatus, ServiceHealth, known_service_health,
};
use axum::Json;

pub(crate) mod embedding;
#[cfg(feature = "fastembed")]
pub(crate) mod events;
#[cfg(feature = "fastembed")]
pub(crate) mod models;

pub(crate) async fn healthz() -> Json<ServiceHealth> {
    Json(known_service_health(
        EMBEDDING_SERVICE_NAME,
        env!("CARGO_PKG_VERSION"),
        HealthStatus::Healthy,
    ))
}
