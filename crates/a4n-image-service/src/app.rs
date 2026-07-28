use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
};

use crate::handlers::{discovery, events, generation, healthz};

pub(crate) async fn run() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_IMAGE_SERVICE_LOG",
        "a4n_image_service=info,tower_http=warn",
    )?;

    let addr = a4n_sidecar_bootstrap::socket_addr("A4N_IMAGE_SERVICE_ADDR", "127.0.0.1:47645")?;
    let policy = a4n_service_protocol::sidecar_resource_policy("image")
        .expect("image sidecar must have a resource policy");
    let (shutdown_signal, shutdown) = a4n_sidecar_bootstrap::sidecar_shutdown();
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route(
            "/image/generate",
            post(generation::generate).layer(DefaultBodyLimit::max(
                policy.body_limit(a4n_service_protocol::BodyLimitKind::Media),
            )),
        )
        .route(
            "/image/generate/events",
            post(events::generate_events).layer(DefaultBodyLimit::max(
                policy.body_limit(a4n_service_protocol::BodyLimitKind::Streaming),
            )),
        )
        .route("/image/request_preview", post(discovery::request_preview))
        .route("/image/probe", post(discovery::probe))
        .route("/image/models", post(discovery::discover_models))
        .layer(DefaultBodyLimit::max(
            policy.body_limit(a4n_service_protocol::BodyLimitKind::Json),
        ))
        .layer(a4n_sidecar_bootstrap::concurrency_limit(
            policy.max_concurrent_requests,
        ))
        .layer(middleware::from_fn(a4n_sidecar_bootstrap::require_auth))
        .merge(a4n_sidecar_bootstrap::shutdown_router(shutdown_signal));
    let app =
        a4n_sidecar_bootstrap::with_runtime_metrics(app, a4n_service_protocol::IMAGE_SERVICE_NAME)?;

    tracing::info!(%addr, "starting image sidecar");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}
