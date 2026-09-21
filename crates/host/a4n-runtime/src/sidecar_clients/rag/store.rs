use std::sync::Arc;

use a4n_error::AppError;
use a4n_rag_client::RagClient;
use a4n_rag_core::vector_store::VectorStore;

mod trait_impl;
mod transport;

pub(crate) fn create_sidecar_rag_store(
    client: Option<Arc<dyn RagClient>>,
) -> Result<Arc<dyn VectorStore>, AppError> {
    let client = client.ok_or_else(|| {
        AppError::ConfigError("RAG sidecar capability 未配置，无法启用 sidecar profile".into())
    })?;
    Ok(Arc::new(SidecarVectorStore { client }))
}

pub(super) struct SidecarVectorStore {
    pub(super) client: Arc<dyn RagClient>,
}
