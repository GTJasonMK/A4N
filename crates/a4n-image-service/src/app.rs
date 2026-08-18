use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};

use crate::handlers::{discovery, events, healthz};

pub(crate) async fn run() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_IMAGE_SERVICE_LOG",
        "a4n_image_service=info,tower_http=warn",
    )?;

    let addr =
        a4n_sidecar_bootstrap::service_socket_addr(a4n_service_protocol::IMAGE_SERVICE_NAME)?;
    let policy = a4n_service_protocol::sidecar_resource_policy(
        a4n_service_protocol::IMAGE_SERVICE_NAME,
    )
        .expect("image sidecar must have a resource policy");
    let app = Router::new()
        .route(a4n_service_protocol::SIDECAR_HEALTH_PATH, get(healthz))
        .route(
            "/image/generate/events",
            post(events::generate_events).layer(DefaultBodyLimit::max(
                policy.body_limit(a4n_service_protocol::BodyLimitKind::Streaming),
            )),
        )
        .route("/image/request_preview", post(discovery::request_preview))
        .route("/image/models", post(discovery::discover_models))
        .layer(DefaultBodyLimit::max(
            policy.body_limit(a4n_service_protocol::BodyLimitKind::Json),
        ))
        .layer(a4n_sidecar_bootstrap::concurrency_limit(
            policy.max_concurrent_requests,
        ));

    tracing::info!(%addr, "starting image sidecar");
    a4n_sidecar_bootstrap::serve_sidecar(app, addr, a4n_service_protocol::IMAGE_SERVICE_NAME).await
}
