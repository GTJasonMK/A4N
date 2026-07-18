use std::sync::Arc;

use a4n_capabilities::A4nCapabilities;
use a4n_core::error::AppError;
use a4n_rag_core::vector_store::VectorStore;

mod outputs;
mod trait_impl;
mod transport;

pub(crate) fn create_sidecar_rag_store(
    capabilities: &A4nCapabilities,
) -> Result<Arc<dyn VectorStore>, AppError> {
    let client = capabilities.rag.clone().ok_or_else(|| {
        AppError::ConfigError("RAG sidecar capability 未配置，无法启用 sidecar profile".into())
    })?;
    Ok(Arc::new(SidecarVectorStore { client }))
}

pub(super) struct SidecarVectorStore {
    pub(super) client: Arc<dyn a4n_capabilities::RagClient>,
}
