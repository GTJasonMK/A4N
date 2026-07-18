use a4n_service_protocol::{
    HealthStatus, IMAGE_PROVIDER_CAPABILITY_VERSION as IMAGE_CAPABILITY_VERSION,
    IMAGE_SERVICE_NAME, PROTOCOL_VERSION, ServiceHealth, ServiceVersion,
};
use axum::Json;

pub(crate) mod discovery;
pub(crate) mod events;
pub(crate) mod generation;

pub(crate) async fn healthz() -> Json<ServiceHealth> {
    Json(ServiceHealth {
        service: IMAGE_SERVICE_NAME.to_string(),
        status: HealthStatus::Healthy,
        version: ServiceVersion {
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: PROTOCOL_VERSION,
            capability_version: IMAGE_CAPABILITY_VERSION.to_string(),
        },
    })
}
