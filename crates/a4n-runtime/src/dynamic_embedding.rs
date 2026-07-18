//! 动态 Embedding Provider：每次调用时解析当前默认嵌入配置。

mod contracts;
mod factory;

use std::path::PathBuf;
use std::sync::Arc;

use a4n_core::config::app_config;
use a4n_core::error::AppError;
use a4n_embedding::provider::{EmbeddingBackend, EmbeddingModelIdentity, EmbeddingProvider};
use a4n_orchestrator::OrchestratorContext;
use a4n_orchestrator::task_queue::PersistentTaskQueue;
use a4n_queue::priority::{TaskPriority, TaskType};
use async_trait::async_trait;

use crate::runtime_config;

pub use contracts::{
    default_fastembed_cache_dir, effective_fastembed_cache_dir, model_identity_from_config,
};
pub(crate) use contracts::{positive_usize, required_text};
use factory::make_provider_from_default_model;
pub use factory::make_provider_from_model;

pub struct DynamicEmbeddingProvider {
    orchestrator: OrchestratorContext,
    task_queue: Arc<PersistentTaskQueue>,
    metadata_config: app_config::EmbeddingConfig,
    fastembed_cache_dir: PathBuf,
}

impl DynamicEmbeddingProvider {
    pub fn new(
        orchestrator: OrchestratorContext,
        task_queue: Arc<PersistentTaskQueue>,
        metadata_config: app_config::EmbeddingConfig,
        fastembed_cache_dir: PathBuf,
    ) -> Self {
        Self {
            orchestrator,
            task_queue,
            metadata_config,
            fastembed_cache_dir,
        }
    }

    async fn resolve_default(&self) -> Result<Arc<dyn EmbeddingProvider>, AppError> {
        if let Some(model) = runtime_config::current_embedding_model() {
            return make_provider_from_model(&model, self.fastembed_cache_dir.clone());
        }

        let model = self
            .orchestrator
            .default_embedding_config_optional()
            .await?;

        make_provider_from_default_model(model, self.fastembed_cache_dir.clone())
    }
}

#[async_trait]
impl EmbeddingProvider for DynamicEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        let provider = self.resolve_default().await?;
        self.task_queue
            .run_tracked(
                TaskType::EmbeddingCall,
                TaskPriority::Low,
                None,
                || async move { provider.embed(texts).await },
            )
            .await
    }

    fn dimension(&self) -> usize {
        self.metadata_config.dimension
    }

    fn model_name(&self) -> &str {
        "dynamic-default-embedding"
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::Dynamic
    }

    fn batch_size(&self) -> usize {
        self.metadata_config.batch_size
    }

    async fn model_identity(&self) -> Result<EmbeddingModelIdentity, AppError> {
        self.resolve_default().await?.model_identity().await
    }

    async fn embed_with_identity(
        &self,
        texts: Vec<String>,
    ) -> Result<(Vec<Vec<f32>>, EmbeddingModelIdentity), AppError> {
        let provider = self.resolve_default().await?;
        self.task_queue
            .run_tracked(
                TaskType::EmbeddingCall,
                TaskPriority::Low,
                None,
                || async move {
                    let identity = provider.model_identity().await?;
                    let embeddings = provider.embed(texts).await?;
                    Ok((embeddings, identity))
                },
            )
            .await
    }

    async fn test_connection(&self) -> Result<bool, AppError> {
        let provider = self.resolve_default().await?;
        self.task_queue
            .run_tracked(
                TaskType::EmbeddingCall,
                TaskPriority::High,
                None,
                || async move { provider.test_connection().await },
            )
            .await
    }
}

#[cfg(test)]
mod tests;
