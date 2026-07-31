#[cfg(not(test))]
use axum::middleware;
use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};

use crate::handlers::{discover_models, healthz, probe, stream_chat};

pub(crate) fn build_router() -> Router {
    let policy = a4n_service_protocol::sidecar_resource_policy("llm")
        .expect("llm sidecar must have a resource policy");
    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/llm/stream", post(stream_chat))
        .route("/llm/probe", post(probe))
        .route("/llm/models", post(discover_models))
        .layer(DefaultBodyLimit::max(
            policy.body_limit(a4n_service_protocol::BodyLimitKind::Json),
        ))
        .layer(a4n_sidecar_bootstrap::concurrency_limit(
            policy.max_concurrent_requests,
        ));
    #[cfg(not(test))]
    let router = router.layer(middleware::from_fn(a4n_sidecar_bootstrap::require_auth));
    router
}

pub(crate) async fn run() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_LLM_GATEWAY_LOG",
        "a4n_llm_gateway=info,tower_http=warn",
    )?;

    let addr = a4n_sidecar_bootstrap::socket_addr("A4N_LLM_GATEWAY_ADDR", "127.0.0.1:47642")?;

    tracing::info!(%addr, "starting llm gateway sidecar");
    a4n_sidecar_bootstrap::serve_sidecar(
        build_router(),
        addr,
        a4n_service_protocol::LLM_SERVICE_NAME,
    )
    .await
}
