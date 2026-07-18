use crate::handlers::{
    collection_exists, completeness_report, completeness_report_by_ids, count_records,
    delete_records, drop_collection, health_check, healthz, ingest, ingest_events,
    list_collections, metadata_by_filter, metadata_by_ids, query, search, search_text,
    update_metadata,
};
use a4n_core::config::env::optional_non_empty_env_path;
use a4n_rag_core::vector_store::VectorStore;
use a4n_rag_index::{
    hybrid_store::HybridVectorStore, lance_store::LanceVectorStore, tantivy_index::TantivyTextIndex,
};
use axum::{Router, routing::get, routing::post};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};

#[derive(Clone)]
pub(crate) struct RagServiceState {
    pub(crate) store: Arc<dyn VectorStore>,
}

pub(crate) async fn run() -> anyhow::Result<()> {
    init_tracing()?;

    let data_dir = data_dir()?;
    let lance = Arc::new(LanceVectorStore::open(data_dir.join("lance")).await?);
    let tantivy = Arc::new(TantivyTextIndex::new(data_dir.join("tantivy"))?);
    let store: Arc<dyn VectorStore> = Arc::new(HybridVectorStore::new(lance, tantivy));
    let state = RagServiceState { store };
    let addr = service_addr()?;
    let app = router(state);

    tracing::info!(%addr, data_dir = ?data_dir, "starting rag sidecar");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_RAG_SERVICE_LOG",
        "a4n_rag_service=info,tower_http=warn",
    )
}

fn data_dir() -> anyhow::Result<PathBuf> {
    Ok(optional_non_empty_env_path("A4N_RAG_SERVICE_DATA_DIR")?
        .unwrap_or_else(|| PathBuf::from("data")))
}

fn service_addr() -> anyhow::Result<SocketAddr> {
    a4n_sidecar_bootstrap::socket_addr("A4N_RAG_SERVICE_ADDR", "127.0.0.1:47644")
}

fn router(state: RagServiceState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/rag/query", post(query))
        .route("/rag/search_text", post(search_text))
        .route("/rag/ingest", post(ingest))
        .route("/rag/ingest/events", post(ingest_events))
        .route("/rag/search", post(search))
        .route("/rag/delete", post(delete_records))
        .route("/rag/collection_exists", post(collection_exists))
        .route("/rag/count", post(count_records))
        .route("/rag/drop_collection", post(drop_collection))
        .route("/rag/list_collections", post(list_collections))
        .route("/rag/update_metadata", post(update_metadata))
        .route("/rag/metadata_by_ids", post(metadata_by_ids))
        .route("/rag/metadata_by_filter", post(metadata_by_filter))
        .route("/rag/health_check", post(health_check))
        .route("/rag/completeness_report", post(completeness_report))
        .route(
            "/rag/completeness_report_by_ids",
            post(completeness_report_by_ids),
        )
        .with_state(state)
}
