mod contracts;
mod events;
mod handlers;
mod protocol;
mod providers;
#[cfg(test)]
mod tests;

use a4n_service_protocol::{
    HealthStatus, SIDECAR_HEALTH_PATH, ServiceHealth, VIDEO_SERVICE_NAME, known_service_health,
};
use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_VIDEO_SERVICE_LOG",
        "a4n_video_service=info,tower_http=warn",
    )?;

    let addr = a4n_sidecar_bootstrap::service_socket_addr(VIDEO_SERVICE_NAME)?;
    let policy = a4n_service_protocol::sidecar_resource_policy(VIDEO_SERVICE_NAME)
        .expect("video sidecar must have a resource policy");
    let app = Router::new()
        .route(SIDECAR_HEALTH_PATH, get(healthz))
        .route(
            "/video/generate/events",
            post(handlers::generate_events).layer(DefaultBodyLimit::max(
                policy.body_limit(a4n_service_protocol::BodyLimitKind::Streaming),
            )),
        )
        .route("/video/poll", post(handlers::poll))
        .route("/video/status", post(handlers::status))
        .route("/video/delete", post(handlers::delete))
        .route("/video/request_preview", post(handlers::request_preview))
        .route("/video/models", post(handlers::discover_models))
        .layer(DefaultBodyLimit::max(
            policy.body_limit(a4n_service_protocol::BodyLimitKind::Json),
        ))
        .layer(a4n_sidecar_bootstrap::concurrency_limit(
            policy.max_concurrent_requests,
        ));

    tracing::info!(%addr, "starting video sidecar");
    a4n_sidecar_bootstrap::serve_sidecar(app, addr, VIDEO_SERVICE_NAME).await
}

async fn healthz() -> Json<ServiceHealth> {
    Json(known_service_health(
        VIDEO_SERVICE_NAME,
        env!("CARGO_PKG_VERSION"),
        HealthStatus::Healthy,
    ))
}
