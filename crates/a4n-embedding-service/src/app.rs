use axum::{Router, routing::get, routing::post};

use crate::handlers::{embedding, events, healthz, models};

pub(crate) async fn run() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_EMBEDDING_SERVICE_LOG",
        "a4n_embedding_service=info,tower_http=warn",
    )?;
    let addr = a4n_sidecar_bootstrap::socket_addr("A4N_EMBEDDING_SERVICE_ADDR", "127.0.0.1:47643")?;
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/embedding/embed_texts", post(embedding::embed_texts))
        .route("/embedding/models", post(models::list_models))
        .route(
            "/embedding/model/download_info",
            post(models::model_download_info),
        )
        .route("/embedding/model/download", post(models::download_model))
        .route(
            "/embedding/model/download/events",
            post(events::download_model_events),
        )
        .route("/embedding/model/delete", post(models::delete_model));
    tracing::info!(%addr, "starting embedding sidecar");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
