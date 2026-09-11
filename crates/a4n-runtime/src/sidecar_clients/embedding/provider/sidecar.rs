use std::sync::Arc;

use a4n_capabilities::EmbeddingClient;
use a4n_domain::ProviderError;
use a4n_embedding::provider::{EmbeddingBackend, EmbeddingModelIdentity, EmbeddingProvider};
use a4n_error::AppError;
use a4n_provider_contract::EmbeddingProviderConfig;
use a4n_queue::priority::{TaskKind, TaskPriority};
use async_trait::async_trait;

use crate::provider_error::from_app_error;
use crate::provider_runtime::DynamicProviderFacade;
use crate::{dynamic_embedding, runtime_config};

use super::super::super::{
    global_provider_service_request, response_payload, service_app_error, service_error_app_error,
};
use super::super::contracts::{EmbedTextsOutput, embedding_texts_payload};

pub(in crate::sidecar_clients::embedding) struct SidecarEmbeddingProvider {
    metadata_dimension: usize,
    metadata_batch_size: usize,
    facade: DynamicProviderFacade,
    client: Arc<dyn EmbeddingClient>,
}

impl SidecarEmbeddingProvider {
    pub(super) fn new(
        metadata_dimension: usize,
        metadata_batch_size: usize,
        facade: DynamicProviderFacade,
        client: Arc<dyn EmbeddingClient>,
    ) -> Self {
        Self {
            metadata_dimension,
            metadata_batch_size,
            facade,
            client,
        }
    }

    async fn resolve_default_model(&self) -> Result<EmbeddingProviderConfig, AppError> {
        if let Some(model) = runtime_config::current_embedding_provider_config() {
            return Ok(model);
        }

        self.facade.default_embedding_config().await
    }

    async fn embed_via_sidecar(
        &self,
        model: EmbeddingProviderConfig,
        texts: Vec<String>,
    ) -> Result<(Vec<Vec<f32>>, EmbeddingModelIdentity), AppError> {
        let payload = embedding_texts_payload(&model, texts)?;
        let expected_dimension = payload.dimension;
        let client = self.client.clone();
        let response = self
            .facade
            .queue_run_tracked(
                TaskKind::EmbeddingCall,
                TaskPriority::Low,
                None,
                || async move {
                    let request = global_provider_service_request(
                        a4n_service_protocol::EMBEDDING_CAPABILITY_VERSION,
                        payload,
                    )
                    .map_err(service_app_error)?;
                    client.embed(request).await.map_err(service_error_app_error)
                },
            )
            .await?;
        let output: EmbedTextsOutput =
            response_payload(response, "embedding").map_err(service_app_error)?;
        if output.identity.dimension != expected_dimension {
            return Err(AppError::ConfigError(format!(
                "嵌入配置 {} 的向量维度为 {}，但 sidecar 模型 {} 实际输出 {} 维",
                model.name, expected_dimension, output.identity.model, output.identity.dimension
            )));
        }
        Ok((output.embeddings, output.identity))
    }
}

#[async_trait]
impl EmbeddingProvider for SidecarEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, ProviderError> {
        self.embed_with_identity(texts)
            .await
            .map(|(embeddings, _)| embeddings)
    }

    fn dimension(&self) -> usize {
        self.metadata_dimension
    }

    fn model_name(&self) -> &str {
        "sidecar-dynamic-embedding"
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::Dynamic
    }

    fn batch_size(&self) -> usize {
        self.metadata_batch_size
    }

    async fn model_identity(&self) -> Result<EmbeddingModelIdentity, ProviderError> {
        let model = self.resolve_default_model().await.map_err(from_app_error)?;
        dynamic_embedding::model_identity_from_config(&model).map_err(from_app_error)
    }

    async fn embed_with_identity(
        &self,
        texts: Vec<String>,
    ) -> Result<(Vec<Vec<f32>>, EmbeddingModelIdentity), ProviderError> {
        let model = self.resolve_default_model().await.map_err(from_app_error)?;
        self.embed_via_sidecar(model, texts)
            .await
            .map_err(from_app_error)
    }
}
