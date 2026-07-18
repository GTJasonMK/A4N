use axum::{Router, routing::get, routing::post};

use crate::handlers::{discovery, events, generation, healthz};

pub(crate) async fn run() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_IMAGE_SERVICE_LOG",
        "a4n_image_service=info,tower_http=warn",
    )?;

    let addr = a4n_sidecar_bootstrap::socket_addr("A4N_IMAGE_SERVICE_ADDR", "127.0.0.1:47645")?;
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/image/generate", post(generation::generate))
        .route("/image/generate/events", post(events::generate_events))
        .route("/image/request_preview", post(discovery::request_preview))
        .route("/image/probe", post(discovery::probe))
        .route("/image/models", post(discovery::discover_models));

    tracing::info!(%addr, "starting image sidecar");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
