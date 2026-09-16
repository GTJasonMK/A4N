use std::sync::Arc;

use a4n_domain::ProviderError;
use a4n_domain::cost::{
    ModelCapability, ModelUsageMeter, ModelUsageRecordCompletion, ModelUsageRecordStart,
};
use a4n_embedding_client::EmbeddingClient;
use a4n_embedding_provider_contract::{
    EmbeddingBackend, EmbeddingModelIdentity, EmbeddingProvider,
};
use a4n_error::AppError;
use a4n_provider_contract::EmbeddingProviderConfig;
use a4n_queue::priority::{TaskKind, TaskPriority};
use async_trait::async_trait;

use crate::provider_error::from_app_error;
use crate::provider_runtime::DynamicProviderFacade;
use crate::{dynamic_embedding, runtime_config};

use super::super::super::{
    global_typed_provider_service_request, service_app_error, service_error_app_error,
    typed_response_payload,
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
        let estimated_tokens = texts
            .iter()
            .map(|text| (text.chars().count() as u64 / 4).max(1))
            .sum();
        let payload = embedding_texts_payload(&model, texts)?;
        let expected_dimension = payload.dimension;
        let cost_record = self
            .facade
            .start_model_usage(ModelUsageRecordStart {
                request_id: uuid::Uuid::new_v4().to_string(),
                user_id: runtime_config::current_user_id(),
                project_id: None,
                root_project_id: None,
                workflow_run_id: None,
                generation_run_id: None,
                workflow: "embedding".to_string(),
                step_key: "embed".to_string(),
                call_key: None,
                source: Some("embedding_call".to_string()),
                capability: ModelCapability::Embedding,
                provider_config_id: Some(model.id),
                provider: Some(model.backend.clone()),
                model: Some(model.model.clone()),
                pricing: model.pricing.clone(),
            })
            .await?;
        let client = self.client.clone();
        let response = self
            .facade
            .queue_run_tracked(
                TaskKind::EmbeddingCall,
                TaskPriority::Low,
                None,
                || async move {
                    let request = global_typed_provider_service_request(
                        a4n_service_protocol::EMBEDDING_CAPABILITY_VERSION,
                        payload,
                    )
                    .map_err(service_app_error)?;
                    client.embed(request).await.map_err(service_error_app_error)
                },
            )
            .await;
        let status = if response.is_ok() {
            "completed"
        } else {
            "failed"
        };
        self.facade
            .finish_model_usage(
                cost_record,
                ModelUsageRecordCompletion {
                    status: status.to_string(),
                    meter: ModelUsageMeter {
                        input_tokens: Some(estimated_tokens),
                        estimated: true,
                        ..Default::default()
                    },
                    error_kind: response.as_ref().err().map(|_| "provider".to_string()),
                    error_message: response.as_ref().err().map(ToString::to_string),
                },
            )
            .await?;
        let response = response?;
        let output: EmbedTextsOutput =
            typed_response_payload(response, "embedding").map_err(service_app_error)?;
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
