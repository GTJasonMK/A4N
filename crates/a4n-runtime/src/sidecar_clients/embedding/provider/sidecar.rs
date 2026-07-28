use std::sync::Arc;

use a4n_capabilities::EmbeddingClient;
use a4n_core::{error::AppError, models::ai::embedding_config};
use a4n_embedding::provider::{EmbeddingBackend, EmbeddingModelIdentity, EmbeddingProvider};
use a4n_orchestrator::DynamicProviderFacade;
use a4n_queue::priority::{TaskKind, TaskPriority};
use async_trait::async_trait;

use crate::{dynamic_embedding, runtime_config};

use super::super::super::{
    global_service_request_without_secrets, service_app_error, service_error_app_error,
    service_response_payload,
};
use super::super::contracts::{
    EmbeddingTextsOutput, EmbeddingTextsPayload, positive_usize, required_text,
};

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

    async fn resolve_default_model(&self) -> Result<embedding_config::Model, AppError> {
        if let Some(model) = runtime_config::current_embedding_model() {
            return Ok(model);
        }

        self.facade.default_embedding_config().await
    }

    pub(in crate::sidecar_clients::embedding) fn validate_sidecar_backend(
        model: &embedding_config::Model,
    ) -> Result<String, AppError> {
        match model.backend.trim().to_ascii_lowercase().as_str() {
            "fastembed" => Ok("fastembed".to_string()),
            "local_hash" => Ok("local_hash".to_string()),
            _ => Err(Self::unsupported_sidecar_backend_error(model)),
        }
    }

    fn unsupported_sidecar_backend_error(model: &embedding_config::Model) -> AppError {
        AppError::ConfigError(format!(
            "embedding sidecar 仅支持 fastembed/local_hash 后端，当前配置 {} 使用 backend={}；API embedding 必须由受信 provider sidecar 单独管理，不能回退到 host embedded provider",
            model.name,
            model.backend.trim()
        ))
    }

    fn embedding_payload(
        &self,
        model: &embedding_config::Model,
        texts: Vec<String>,
    ) -> Result<EmbeddingTextsPayload, AppError> {
        Ok(EmbeddingTextsPayload {
            backend: Self::validate_sidecar_backend(model)?,
            model: required_text(&model.model, "模型名", &model.name)?,
            dimension: positive_usize(model.dimension, "向量维度", &model.name)?,
            batch_size: positive_usize(model.batch_size, "批量大小", &model.name)?,
            texts,
        })
    }

    async fn embed_via_sidecar(
        &self,
        model: embedding_config::Model,
        texts: Vec<String>,
    ) -> Result<(Vec<Vec<f32>>, EmbeddingModelIdentity), AppError> {
        let expected_dimension = positive_usize(model.dimension, "向量维度", &model.name)?;
        let payload = self.embedding_payload(&model, texts)?;
        let client = self.client.clone();
        let response = self
            .facade
            .queue_run_tracked(
                TaskKind::EmbeddingCall,
                TaskPriority::Low,
                None,
                || async move {
                    let request = global_service_request_without_secrets(
                        a4n_service_protocol::EMBEDDING_CAPABILITY_VERSION,
                        payload,
                    )
                    .map_err(service_app_error)?;
                    client.embed(request).await.map_err(service_error_app_error)
                },
            )
            .await?;
        let output: EmbeddingTextsOutput =
            service_response_payload(response, "embedding").map_err(service_app_error)?;
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
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        let model = self.resolve_default_model().await?;
        self.embed_via_sidecar(model, texts)
            .await
            .map(|(items, _)| items)
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

    async fn model_identity(&self) -> Result<EmbeddingModelIdentity, AppError> {
        let model = self.resolve_default_model().await?;
        Self::validate_sidecar_backend(&model)?;
        dynamic_embedding::model_identity_from_config(&model)
    }

    async fn embed_with_identity(
        &self,
        texts: Vec<String>,
    ) -> Result<(Vec<Vec<f32>>, EmbeddingModelIdentity), AppError> {
        let model = self.resolve_default_model().await?;
        self.embed_via_sidecar(model, texts).await
    }

    async fn test_connection(&self) -> Result<bool, AppError> {
        self.embed_one("测试".to_string()).await.map(|_| true)
    }
}
