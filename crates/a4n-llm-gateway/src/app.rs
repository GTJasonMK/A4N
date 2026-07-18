use axum::{
    Router,
    routing::{get, post},
};

use crate::handlers::{chat, discover_models, healthz, probe, stream_chat};

pub(crate) fn build_router() -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/llm/chat", post(chat))
        .route("/llm/stream", post(stream_chat))
        .route("/llm/probe", post(probe))
        .route("/llm/models", post(discover_models))
}

pub(crate) async fn run() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_LLM_GATEWAY_LOG",
        "a4n_llm_gateway=info,tower_http=warn",
    )?;

    let addr = a4n_sidecar_bootstrap::socket_addr("A4N_LLM_GATEWAY_ADDR", "127.0.0.1:47642")?;

    tracing::info!(%addr, "starting llm gateway sidecar");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, build_router()).await?;
    Ok(())
}
