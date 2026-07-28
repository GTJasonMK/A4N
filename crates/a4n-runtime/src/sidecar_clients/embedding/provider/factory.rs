use std::sync::Arc;

use a4n_capabilities::A4nCapabilities;
use a4n_core::{config::app_config::AppConfig, error::AppError};
use a4n_embedding::provider::EmbeddingProvider;
use a4n_orchestrator::DynamicProviderFacade;

use crate::{
    dynamic_embedding::{self, DynamicEmbeddingProvider},
    state::RuntimeProfile,
};

use super::super::super::required_sidecar_client;
use super::SidecarEmbeddingProvider;

pub(crate) fn create_embedding_provider(
    facade: DynamicProviderFacade,
    config: &AppConfig,
    profile: RuntimeProfile,
    capabilities: &A4nCapabilities,
) -> Result<Arc<dyn EmbeddingProvider>, AppError> {
    let sidecar_client =
        required_sidecar_client(profile, capabilities.embedding.as_ref(), "embedding")?;

    if let Some(client) = sidecar_client {
        return Ok(Arc::new(SidecarEmbeddingProvider::new(
            config.embedding.dimension,
            config.embedding.batch_size,
            facade,
            client,
        )));
    }
    Ok(Arc::new(DynamicEmbeddingProvider::new(
        facade,
        config.embedding.clone(),
        dynamic_embedding::default_fastembed_cache_dir(config),
    )))
}
