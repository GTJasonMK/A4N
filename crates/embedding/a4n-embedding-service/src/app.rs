use a4n_service_protocol::{
    EMBEDDING_EMBED_TEXTS_PATH, EMBEDDING_MODEL_DELETE_PATH, EMBEDDING_MODEL_DOWNLOAD_EVENTS_PATH,
    EMBEDDING_MODEL_DOWNLOAD_INFO_PATH, EMBEDDING_MODELS_PATH,
};
use axum::{
    Router,
    extract::DefaultBodyLimit,
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
    let addr = a4n_sidecar_bootstrap::socket_addr(
        a4n_embedding_contract::EMBEDDING_SERVICE_ADDR_ENV,
        a4n_embedding_contract::EMBEDDING_SERVICE_DEFAULT_ADDR,
    )?;
    let policy =
        a4n_service_protocol::sidecar_resource_policy(a4n_service_protocol::EMBEDDING_SERVICE_NAME)
            .expect("embedding sidecar must have a resource policy");
    let app = Router::new()
        .route(a4n_service_protocol::SIDECAR_HEALTH_PATH, get(healthz))
        .route(EMBEDDING_EMBED_TEXTS_PATH, post(embedding::embed_texts));
    #[cfg(feature = "fastembed")]
    let app = app
        .route(
            EMBEDDING_MODELS_PATH,
            post(crate::handlers::models::list_models),
        )
        .route(
            EMBEDDING_MODEL_DOWNLOAD_INFO_PATH,
            post(crate::handlers::models::model_download_info),
        )
        .route(
            EMBEDDING_MODEL_DOWNLOAD_EVENTS_PATH,
            post(crate::handlers::events::download_model_events),
        )
        .route(
            EMBEDDING_MODEL_DELETE_PATH,
            post(crate::handlers::models::delete_model),
        );
    let app = app
        .layer(DefaultBodyLimit::max(
            policy.body_limit(a4n_service_protocol::BodyLimitKind::Json),
        ))
        .layer(a4n_sidecar_bootstrap::concurrency_limit(
            policy.max_concurrent_requests,
        ));
    tracing::info!(%addr, "starting embedding sidecar");
    a4n_sidecar_bootstrap::serve_sidecar(app, addr, a4n_service_protocol::EMBEDDING_SERVICE_NAME)
        .await
}
