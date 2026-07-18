use super::disabled::DisabledVectorStore;
use crate::state::profile::RuntimeProfile;
use a4n_core::{config::app_config::AppConfig, error::AppError};
use a4n_rag_core::vector_store::VectorStore;
#[cfg(feature = "embedded-rag-index")]
use a4n_rag_index::{
    hybrid_store::HybridVectorStore, lance_store::LanceVectorStore, tantivy_index::TantivyTextIndex,
};
use std::sync::Arc;

pub(in crate::state) async fn create_rag_store(
    config: &AppConfig,
    runtime_profile: RuntimeProfile,
    capabilities: &a4n_capabilities::A4nCapabilities,
) -> Result<Arc<dyn VectorStore>, AppError> {
    if runtime_profile.uses_sidecars() {
        return crate::sidecar_clients::create_sidecar_rag_store(capabilities);
    }

    if !runtime_profile.initializes_rag() {
        tracing::warn!(
            runtime_profile = runtime_profile.as_str(),
            "RAG vector store is disabled by runtime profile"
        );
        return Ok(Arc::new(DisabledVectorStore::new(runtime_profile)));
    }

    create_embedded_rag_store(config).await
}

#[cfg(feature = "embedded-rag-index")]
async fn create_embedded_rag_store(config: &AppConfig) -> Result<Arc<dyn VectorStore>, AppError> {
    let lance_vec = Arc::new(LanceVectorStore::open(config.data_dir.join("lance")).await?);
    let tantivy_idx = Arc::new(TantivyTextIndex::new(config.data_dir.join("tantivy"))?);
    Ok(Arc::new(HybridVectorStore::new(lance_vec, tantivy_idx)))
}

#[cfg(not(feature = "embedded-rag-index"))]
async fn create_embedded_rag_store(_config: &AppConfig) -> Result<Arc<dyn VectorStore>, AppError> {
    Err(AppError::ConfigError(
        "full runtime profile 需要编译 `embedded-rag-index` feature，不能回退到 disabled RAG"
            .to_string(),
    ))
}
