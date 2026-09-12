use std::sync::Arc;

use crate::provider_runtime::DynamicProviderFacade;
use a4n_config::app_config::AppConfig;
use a4n_embedding::provider::EmbeddingProvider;
use a4n_embedding_client::EmbeddingClient;
use a4n_error::AppError;

use crate::{dynamic_embedding::DynamicEmbeddingProvider, state::RuntimeProfile};

use super::super::super::required_sidecar_client;
use super::SidecarEmbeddingProvider;

pub(crate) fn create_embedding_provider(
    facade: DynamicProviderFacade,
    config: &AppConfig,
    profile: RuntimeProfile,
    client: Option<Arc<dyn EmbeddingClient>>,
) -> Result<Arc<dyn EmbeddingProvider>, AppError> {
    let sidecar_client = required_sidecar_client(profile, client.as_ref(), "embedding")?;

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
        config.embedding.dimension,
        config.embedding.batch_size,
        config.embedding_models_dir(),
    )))
}
