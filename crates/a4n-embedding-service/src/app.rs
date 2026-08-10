use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
};

use crate::handlers::{embedding, healthz};

pub(crate) async fn run() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_EMBEDDING_SERVICE_LOG",
        "a4n_embedding_service=info,tower_http=warn",
    )?;
    #[cfg(feature = "fastembed")]
    {
        let cache_dir = crate::handlers::models::resolve_cache_dir()?;
        a4n_sidecar_bootstrap::ensure_data_directory_manifest(
            &cache_dir,
            "embedding-model-cache",
            "rebuildable_cache",
            1,
            false,
            "delete_and_redownload",
            serde_json::json!({"contents": "fastembed model files"}),
        )?;
        let removed =
            a4n_embedding_fastembed::remove_incomplete_fastembed_model_caches(&cache_dir)?;
        if !removed.is_empty() {
            tracing::warn!(?removed, "removed incomplete fastembed model caches");
        }
    }
    let addr = a4n_sidecar_bootstrap::socket_addr("A4N_EMBEDDING_SERVICE_ADDR", "127.0.0.1:47643")?;
    let policy = a4n_service_protocol::sidecar_resource_policy("embedding")
        .expect("embedding sidecar must have a resource policy");
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/embedding/embed_texts", post(embedding::embed_texts));
    #[cfg(feature = "fastembed")]
    let app = app
        .route(
            "/embedding/models",
            post(crate::handlers::models::list_models),
        )
        .route(
            "/embedding/model/download_info",
            post(crate::handlers::models::model_download_info),
        )
        .route(
            "/embedding/model/download/events",
            post(crate::handlers::events::download_model_events),
        )
        .route(
            "/embedding/model/delete",
            post(crate::handlers::models::delete_model),
        );
    #[cfg(not(feature = "fastembed"))]
    let app = app;
    let app = app
        .layer(DefaultBodyLimit::max(
            policy.body_limit(a4n_service_protocol::BodyLimitKind::Json),
        ))
        .layer(a4n_sidecar_bootstrap::concurrency_limit(
            policy.max_concurrent_requests,
        ))
        .layer(middleware::from_fn(a4n_sidecar_bootstrap::require_auth));
    tracing::info!(%addr, "starting embedding sidecar");
    a4n_sidecar_bootstrap::serve_sidecar(app, addr, a4n_service_protocol::EMBEDDING_SERVICE_NAME)
        .await
}
