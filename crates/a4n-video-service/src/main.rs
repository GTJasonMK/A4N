mod contracts;
mod events;
mod handlers;
mod protocol;
mod providers;
#[cfg(test)]
mod tests;

use a4n_service_protocol::{
    HealthStatus, ServiceHealth, VIDEO_SERVICE_NAME, known_service_health_for_host,
};
use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    http::HeaderMap,
    middleware,
    routing::{get, post},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_VIDEO_SERVICE_LOG",
        "a4n_video_service=info,tower_http=warn",
    )?;

    let addr = a4n_sidecar_bootstrap::socket_addr("A4N_VIDEO_SERVICE_ADDR", "127.0.0.1:47646")?;
    let policy = a4n_service_protocol::sidecar_resource_policy("video")
        .expect("video sidecar must have a resource policy");
    let app = Router::new()
        .route("/healthz", get(healthz))
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
        .route("/video/probe", post(handlers::probe))
        .route("/video/models", post(handlers::discover_models))
        .layer(DefaultBodyLimit::max(
            policy.body_limit(a4n_service_protocol::BodyLimitKind::Json),
        ))
        .layer(a4n_sidecar_bootstrap::concurrency_limit(
            policy.max_concurrent_requests,
        ))
        .layer(middleware::from_fn(a4n_sidecar_bootstrap::require_auth));

    tracing::info!(%addr, "starting video sidecar");
    a4n_sidecar_bootstrap::serve_sidecar(app, addr, VIDEO_SERVICE_NAME).await
}

async fn healthz(headers: HeaderMap) -> Json<ServiceHealth> {
    Json(known_service_health_for_host(
        VIDEO_SERVICE_NAME,
        env!("CARGO_PKG_VERSION"),
        HealthStatus::Healthy,
        a4n_sidecar_bootstrap::host_protocol_version(&headers),
    ))
}
