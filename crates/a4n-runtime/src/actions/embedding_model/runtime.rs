use super::{
    contracts::{LocalEmbedTextsInput, LocalEmbedTextsOutput, LocalEmbeddingConfigInput},
    provider::{
        cached_local_embedding_provider, fastembed_cache_dir_from_config,
        fastembed_identity_from_config,
    },
};
use a4n_core::error::AppError;
use a4n_embedding::provider::EmbeddingProvider;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct LocalEmbeddingProviderRuntime {
    fastembed_cache_dir: PathBuf,
    pub(super) providers: Arc<Mutex<HashMap<String, Arc<dyn EmbeddingProvider>>>>,
}

impl LocalEmbeddingProviderRuntime {
    pub fn from_app_config(config: &a4n_core::config::app_config::AppConfig) -> Self {
        Self::new(fastembed_cache_dir_from_config(config))
    }

    pub fn new(fastembed_cache_dir: PathBuf) -> Self {
        Self {
            fastembed_cache_dir,
            providers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn fastembed_cache_dir(&self) -> PathBuf {
        self.fastembed_cache_dir.clone()
    }

    pub async fn clear_provider_cache(&self) {
        self.providers.lock().await.clear();
    }

    pub async fn test_embedding_config(
        &self,
        input: &LocalEmbeddingConfigInput,
    ) -> Result<bool, AppError> {
        let provider = cached_local_embedding_provider(self, input).await?;
        provider.test_connection().await
    }

    pub async fn embed_texts(
        &self,
        input: LocalEmbedTextsInput,
    ) -> Result<LocalEmbedTextsOutput, AppError> {
        let identity = fastembed_identity_from_config(self, &input.config)?;
        let identity = a4n_api_contract::embedding::EmbeddingModelIdentity {
            backend: identity.backend,
            model: identity.model,
            dimension: identity.dimension,
        };
        if input.texts.is_empty() {
            return Ok(LocalEmbedTextsOutput {
                identity,
                embeddings: Vec::new(),
            });
        }
        let provider = cached_local_embedding_provider(self, &input.config).await?;
        let embeddings = provider.embed(input.texts).await?;
        Ok(LocalEmbedTextsOutput {
            identity,
            embeddings,
        })
    }
}
