use a4n_service_protocol::{
    HealthStatus, IMAGE_SERVICE_NAME, ServiceHealth, known_service_health,
};
use axum::Json;

pub(crate) mod discovery;
pub(crate) mod events;

pub(crate) async fn healthz() -> Json<ServiceHealth> {
    Json(known_service_health(
        IMAGE_SERVICE_NAME,
        env!("CARGO_PKG_VERSION"),
        HealthStatus::Healthy,
    ))
}
