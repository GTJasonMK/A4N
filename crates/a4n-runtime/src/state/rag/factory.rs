use super::disabled::DisabledVectorStore;
use crate::state::profile::RuntimeProfile;
use a4n_error::AppError;
use a4n_rag_core::vector_store::VectorStore;
#[cfg(feature = "embedded-rag-index")]
use a4n_rag_index::QdrantVectorStore;
use std::sync::Arc;

pub(in crate::state) async fn create_rag_store(
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

    create_embedded_rag_store().await
}

async fn create_embedded_rag_store() -> Result<Arc<dyn VectorStore>, AppError> {
    #[cfg(feature = "embedded-rag-index")]
    {
        let store = QdrantVectorStore::connect_from_env().await?;
        return Ok(store as Arc<dyn VectorStore>);
    }

    #[cfg(not(feature = "embedded-rag-index"))]
    Err(AppError::ConfigError(
        "full runtime profile 需要编译 `embedded-rag-index` feature，不能回退到 disabled RAG"
            .to_string(),
    ))
}
