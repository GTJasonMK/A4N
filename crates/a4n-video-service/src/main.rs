mod contracts;
mod events;
mod handlers;
mod protocol;
mod providers;
#[cfg(test)]
mod tests;

use a4n_service_protocol::{
    HealthStatus, PROTOCOL_VERSION, ServiceHealth, ServiceVersion,
    VIDEO_PROVIDER_CAPABILITY_VERSION as VIDEO_CAPABILITY_VERSION, VIDEO_SERVICE_NAME,
};
use axum::{
    Json, Router,
    routing::{get, post},
};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_VIDEO_SERVICE_LOG",
        "a4n_video_service=info,tower_http=warn",
    )?;

    let addr = a4n_sidecar_bootstrap::socket_addr("A4N_VIDEO_SERVICE_ADDR", "127.0.0.1:47646")?;
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/video/generate", post(handlers::generate))
        .route("/video/generate/events", post(handlers::generate_events))
        .route("/video/poll", post(handlers::poll))
        .route("/video/request_preview", post(handlers::request_preview))
        .route("/video/probe", post(handlers::probe))
        .route("/video/models", post(handlers::discover_models));

    tracing::info!(%addr, "starting video sidecar");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn healthz() -> Json<ServiceHealth> {
    Json(ServiceHealth {
        service: VIDEO_SERVICE_NAME.to_string(),
        status: HealthStatus::Healthy,
        version: ServiceVersion {
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: PROTOCOL_VERSION,
            capability_version: VIDEO_CAPABILITY_VERSION.to_string(),
        },
    })
}
