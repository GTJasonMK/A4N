//! 动态 Embedding Provider：每次调用时解析当前默认嵌入配置。

mod contracts;
mod factory;

use std::path::PathBuf;
use std::sync::Arc;

use a4n_domain::ProviderError;
use a4n_domain::cost::{
    ModelCapability, ModelUsageMeter, ModelUsageRecordCompletion, ModelUsageRecordStart,
};
use a4n_embedding_provider_contract::{
    EmbeddingBackend, EmbeddingModelIdentity, EmbeddingProvider,
};
use a4n_error::AppError;
use a4n_queue::priority::{TaskKind, TaskPriority};
use async_trait::async_trait;

use crate::provider_error::from_app_error;
use crate::provider_runtime::DynamicProviderFacade;
use crate::runtime_config;

pub(crate) use contracts::model_identity_from_config;
#[cfg(feature = "local-embedding")]
pub(crate) use contracts::resolve_fastembed_cache_dir;
pub(crate) use contracts::{positive_usize, required_text};
pub(crate) use factory::make_provider_from_model;
use factory::{make_provider_from_default_model, missing_default_embedding_config_error};

pub(crate) fn embedding_request_snapshot(
    model: &str,
    texts: &[String],
    batch_size: usize,
) -> Option<serde_json::Value> {
    if texts.is_empty() || batch_size == 0 {
        return None;
    }
    let requests = texts
        .chunks(batch_size)
        .map(|input| serde_json::json!({ "model": model, "input": input }))
        .collect::<Vec<_>>();
    if requests.len() == 1 {
        requests.into_iter().next()
    } else {
        Some(serde_json::json!({ "requests": requests }))
    }
}

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

    async fn resolve_default(
        &self,
    ) -> Result<
        (
            a4n_provider_contract::EmbeddingProviderConfig,
            Arc<dyn EmbeddingProvider>,
        ),
        AppError,
    > {
        if let Some(model) = runtime_config::current_embedding_provider_config() {
            let provider = make_provider_from_model(&model, self.fastembed_cache_dir.clone())?;
            return Ok((model, provider));
        }

        let model = self.facade.default_embedding_config_optional().await?;
        let model = model.ok_or_else(missing_default_embedding_config_error)?;
        let provider = make_provider_from_default_model(
            Some(model.clone()),
            self.fastembed_cache_dir.clone(),
        )?;
        Ok((model, provider))
    }
}

#[async_trait]
impl EmbeddingProvider for DynamicEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, ProviderError> {
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

    async fn model_identity(&self) -> Result<EmbeddingModelIdentity, ProviderError> {
        self.resolve_default()
            .await
            .map_err(from_app_error)?
            .1
            .model_identity()
            .await
    }

    async fn embed_with_identity(
        &self,
        texts: Vec<String>,
    ) -> Result<(Vec<Vec<f32>>, EmbeddingModelIdentity), ProviderError> {
        let (config, provider) = self.resolve_default().await.map_err(from_app_error)?;
        let expected_count = texts.len();
        let expected_identity = model_identity_from_config(&config).map_err(from_app_error)?;
        let estimated_tokens = texts
            .iter()
            .map(|text| (text.chars().count() as u64 / 4).max(1))
            .sum();
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
                provider_config_id: Some(config.id),
                provider: Some(config.backend.clone()),
                model: Some(config.model.clone()),
                pricing: config.pricing.clone(),
                request_json: crate::dynamic_embedding::embedding_request_snapshot(
                    &config.model,
                    &texts,
                    provider.batch_size(),
                ),
            })
            .await
            .map_err(from_app_error)?;
        let result = self
            .facade
            .queue_run_tracked(
                TaskKind::EmbeddingCall,
                TaskPriority::Low,
                None,
                || async move {
                    provider
                        .embed_with_identity(texts)
                        .await
                        .map_err(AppError::from)
                },
            )
            .await;
        let result = result.and_then(|(embeddings, identity)| {
            a4n_embedding_provider_contract::validate_embedding_output(
                &embeddings,
                &identity,
                &expected_identity,
                expected_count,
            )
            .map_err(AppError::from)?;
            Ok((embeddings, identity))
        });
        let status = if result.is_ok() {
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
                    error_kind: result.as_ref().err().map(|_| "provider".to_string()),
                    error_message: result.as_ref().err().map(ToString::to_string),
                },
            )
            .await
            .map_err(from_app_error)?;
        result.map_err(from_app_error)
    }
}

#[cfg(test)]
mod tests;
