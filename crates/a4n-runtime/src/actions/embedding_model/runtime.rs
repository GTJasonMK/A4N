use super::{
    LocalEmbedTextsInput, LocalEmbedTextsOutput, LocalEmbeddingConfigInput,
    provider::cached_local_embedding_provider,
};
use a4n_core::error::AppError;
use a4n_embedding::provider::EmbeddingProvider;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

pub struct LocalEmbeddingProviderRuntime {
    fastembed_cache_dir: PathBuf,
    pub(super) providers:
        Mutex<HashMap<(String, usize, usize, PathBuf), Arc<dyn EmbeddingProvider>>>,
}

impl LocalEmbeddingProviderRuntime {
    pub fn from_app_config(config: &a4n_core::config::app_config::AppConfig) -> Self {
        Self {
            fastembed_cache_dir: config.embedding_models_dir(),
            providers: Mutex::new(HashMap::new()),
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
    ) -> Result<(), AppError> {
        let provider = cached_local_embedding_provider(self, input).await?;
        provider.test_connection().await
    }

    pub async fn embed_texts(
        &self,
        input: LocalEmbedTextsInput,
    ) -> Result<LocalEmbedTextsOutput, AppError> {
        let provider = cached_local_embedding_provider(self, &input.config).await?;
        let (embeddings, identity) = provider.embed_with_identity(input.texts).await?;
        Ok(LocalEmbedTextsOutput {
            identity,
            embeddings,
        })
    }
}
