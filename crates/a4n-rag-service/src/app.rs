use crate::handlers::{
    completeness_report_by_ids, delete_records, drop_collection, healthz, ingest_events,
    metadata_by_filter, metadata_by_ids, query, search, search_text,
};
use a4n_core::config::env::optional_non_empty_env_path;
use a4n_core::error::AppError;
use a4n_rag_core::vector_store::VectorStore;
use a4n_rag_index::QdrantVectorStore;
use a4n_service_protocol::{
    RAG_COMPLETENESS_REPORT_BY_IDS_PATH, RAG_DELETE_PATH, RAG_DROP_COLLECTION_PATH,
    RAG_INGEST_EVENTS_PATH, RAG_METADATA_BY_FILTER_PATH, RAG_METADATA_BY_IDS_PATH,
    RAG_QUERY_PATH, RAG_SEARCH_PATH, RAG_SEARCH_TEXT_PATH,
};
use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};
use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::Arc,
};

const RAG_MUTATION_MARKER_FILE: &str = ".a4n-rag-mutation-in-progress";

#[derive(Clone)]
pub(crate) struct RagServiceState {
    pub(crate) store: Arc<dyn VectorStore>,
    mutation_lock: Arc<tokio::sync::Mutex<()>>,
    data_dir: PathBuf,
}

impl RagServiceState {
    pub(crate) async fn persist_mutation<T>(
        &self,
        mutation: impl Future<Output = Result<T, AppError>>,
    ) -> Result<T, AppError> {
        let _lock = self.mutation_lock.lock().await;
        let marker = RagMutationMarker::begin(&self.data_dir)?;
        let output = mutation.await;
        marker.complete()?;
        output
    }
}

struct RagMutationMarker {
    path: PathBuf,
}

impl RagMutationMarker {
    fn require_clean(data_dir: &Path) -> anyhow::Result<()> {
        let path = data_dir.join(RAG_MUTATION_MARKER_FILE);
        if path.exists() {
            anyhow::bail!(
                "RAG 索引存在未完成写入标记 {}；请确认 Qdrant 数据一致后删除该标记，并从 canonical source records 显式重建",
                path.display()
            );
        }
        Ok(())
    }

    fn begin(data_dir: &Path) -> Result<Self, AppError> {
        let path = data_dir.join(RAG_MUTATION_MARKER_FILE);
        let file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)?;
        file.sync_all()?;
        Ok(Self { path })
    }

    fn complete(self) -> Result<(), AppError> {
        std::fs::remove_file(self.path)?;
        Ok(())
    }
}

pub(crate) async fn run() -> anyhow::Result<()> {
    init_tracing()?;

    let data_dir = data_dir()?;
    std::fs::create_dir_all(&data_dir)?;
    a4n_sidecar_bootstrap::ensure_data_directory_manifest(
        &data_dir,
        "rag-index",
        "rebuildable_derived_data",
        2,
        false,
        "rebuild_from_source_records",
        serde_json::json!({"engines": ["qdrant"]}),
    )?;
    RagMutationMarker::require_clean(&data_dir)?;
    let store: Arc<dyn VectorStore> = QdrantVectorStore::connect_from_env().await?;
    let state = RagServiceState {
        store,
        mutation_lock: Arc::new(tokio::sync::Mutex::new(())),
        data_dir: data_dir.clone(),
    };
    let addr = a4n_sidecar_bootstrap::service_socket_addr(a4n_service_protocol::RAG_SERVICE_NAME)?;

    tracing::info!(%addr, data_dir = ?data_dir, "starting rag sidecar (qdrant backend)");
    a4n_sidecar_bootstrap::serve_sidecar(
        router(state),
        addr,
        a4n_service_protocol::RAG_SERVICE_NAME,
    )
    .await
}

fn init_tracing() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_RAG_SERVICE_LOG",
        "a4n_rag_service=info,tower_http=warn",
    )
}

fn data_dir() -> Result<PathBuf, AppError> {
    optional_non_empty_env_path("A4N_RAG_SERVICE_DATA_DIR")?.ok_or_else(|| {
        AppError::ConfigError("A4N_RAG_SERVICE_DATA_DIR 未设置".to_string())
    })
}

fn router(state: RagServiceState) -> Router {
    let policy = a4n_service_protocol::sidecar_resource_policy(
        a4n_service_protocol::RAG_SERVICE_NAME,
    )
    .expect("rag sidecar must have a resource policy");
    Router::new()
        .route(a4n_service_protocol::SIDECAR_HEALTH_PATH, get(healthz))
        .route(RAG_QUERY_PATH, post(query))
        .route(RAG_SEARCH_TEXT_PATH, post(search_text))
        .route(RAG_INGEST_EVENTS_PATH, post(ingest_events))
        .route(RAG_SEARCH_PATH, post(search))
        .route(RAG_DELETE_PATH, post(delete_records))
        .route(RAG_DROP_COLLECTION_PATH, post(drop_collection))
        .route(RAG_METADATA_BY_IDS_PATH, post(metadata_by_ids))
        .route(RAG_METADATA_BY_FILTER_PATH, post(metadata_by_filter))
        .route(RAG_COMPLETENESS_REPORT_BY_IDS_PATH, post(completeness_report_by_ids))
        .layer(DefaultBodyLimit::max(
            policy.body_limit(a4n_service_protocol::BodyLimitKind::Json),
        ))
        .layer(a4n_sidecar_bootstrap::concurrency_limit(
            policy.max_concurrent_requests,
        ))
        .with_state(state)
}

#[cfg(test)]
mod mutation_marker_tests {
    use super::{RagMutationMarker, RagServiceState};
    use a4n_core::error::AppError;
    use a4n_rag_core::memory_store::MemoryVectorStore;
    use a4n_test_support::process::SidecarProcessTestDir;
    use std::sync::Arc;

    #[test]
    fn interrupted_mutation_marker_survives_drop_until_explicit_rebuild() {
        let data_dir = SidecarProcessTestDir::new("rag-mutation-marker");
        let marker = RagMutationMarker::begin(data_dir.path()).unwrap();
        drop(marker);
        assert!(RagMutationMarker::require_clean(data_dir.path()).is_err());
    }

    #[tokio::test]
    async fn handled_mutation_error_clears_marker() {
        let data_dir = SidecarProcessTestDir::new("rag-mutation-error");
        let state = RagServiceState {
            store: Arc::new(MemoryVectorStore::new()),
            mutation_lock: Arc::new(tokio::sync::Mutex::new(())),
            data_dir: data_dir.path().to_path_buf(),
        };

        let error = state
            .persist_mutation(async {
                Err::<(), _>(AppError::ConfigError("expected".into()))
            })
            .await
            .expect_err("mutation should fail");

        assert!(matches!(error, AppError::ConfigError(message) if message == "expected"));
        assert!(RagMutationMarker::require_clean(data_dir.path()).is_ok());
    }
}
