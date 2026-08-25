//! 动态 Embedding Provider：每次调用时解析当前默认嵌入配置。

mod contracts;
mod factory;

use std::path::PathBuf;
use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_embedding::provider::{EmbeddingBackend, EmbeddingModelIdentity, EmbeddingProvider};
use a4n_orchestrator::DynamicProviderFacade;
use a4n_queue::priority::{TaskKind, TaskPriority};
use async_trait::async_trait;

use crate::runtime_config;

pub(crate) use contracts::model_identity_from_config;
#[cfg(feature = "local-embedding")]
pub(crate) use contracts::resolve_fastembed_cache_dir;
pub(crate) use contracts::{positive_usize, required_text};
use factory::make_provider_from_default_model;
pub(crate) use factory::make_provider_from_model;

pub(crate) struct DynamicEmbeddingProvider {
    facade: DynamicProviderFacade,
    metadata_dimension: usize,
    metadata_batch_size: usize,
    fastembed_cache_dir: PathBuf,
}

impl DynamicEmbeddingProvider {
    pub(crate) fn new(
        facade: DynamicProviderFacade,
        metadata_dimension: usize,
        metadata_batch_size: usize,
        fastembed_cache_dir: PathBuf,
    ) -> Self {
        Self {
            facade,
            metadata_dimension,
            metadata_batch_size,
            fastembed_cache_dir,
        }
    }

    async fn resolve_default(&self) -> Result<Arc<dyn EmbeddingProvider>, AppError> {
        if let Some(model) = runtime_config::current_embedding_model() {
            return make_provider_from_model(&model, self.fastembed_cache_dir.clone());
        }

        let model = self.facade.default_embedding_config_optional().await?;

        make_provider_from_default_model(model, self.fastembed_cache_dir.clone())
    }
}

#[async_trait]
impl EmbeddingProvider for DynamicEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        self.embed_with_identity(texts)
            .await
            .map(|(embeddings, _)| embeddings)
    }

    fn dimension(&self) -> usize {
        self.metadata_dimension
    }

    fn model_name(&self) -> &str {
        "dynamic-default-embedding"
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::Dynamic
    }

    fn batch_size(&self) -> usize {
        self.metadata_batch_size
    }

    async fn model_identity(&self) -> Result<EmbeddingModelIdentity, AppError> {
        self.resolve_default().await?.model_identity().await
    }

    async fn embed_with_identity(
        &self,
        texts: Vec<String>,
    ) -> Result<(Vec<Vec<f32>>, EmbeddingModelIdentity), AppError> {
        let provider = self.resolve_default().await?;
        self.facade
            .queue_run_tracked(
                TaskKind::EmbeddingCall,
                TaskPriority::Low,
                None,
                || async move { provider.embed_with_identity(texts).await },
            )
            .await
    }
}

#[cfg(test)]
mod tests;
