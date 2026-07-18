use a4n_service_protocol::{
    EMBEDDING_CAPABILITY_VERSION, EMBEDDING_SERVICE_NAME, HealthStatus, PROTOCOL_VERSION,
    ServiceHealth, ServiceVersion,
};
use axum::Json;

pub(crate) mod embedding;
pub(crate) mod events;
pub(crate) mod models;

pub(crate) async fn healthz() -> Json<ServiceHealth> {
    Json(ServiceHealth {
        service: EMBEDDING_SERVICE_NAME.to_string(),
        status: HealthStatus::Healthy,
        version: ServiceVersion {
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: PROTOCOL_VERSION,
            capability_version: EMBEDDING_CAPABILITY_VERSION.to_string(),
        },
    })
}
