use crate::dynamic_llm::config::{model_capabilities, validated_timeout_secs};
use a4n_error::AppError;
use a4n_llm_config::LlmConfigSnapshot;
use a4n_llm_provider_contract::provider::{ChatChunk, ChatRequest, LlmProvider, ModelCapabilities};
use a4n_llm_sidecar_contract::LlmChatRequest;
use a4n_llm_sidecar_contract::{LlmGatewayPayload, LlmGatewayProviderConfig, LlmProbePayload};
use a4n_provider_contract::LlmProviderConfig;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

#[derive(Debug, Clone)]
pub(in crate::dynamic_llm) struct ResolvedLlmModel {
    pub(in crate::dynamic_llm) model: LlmProviderConfig,
    pub(in crate::dynamic_llm) timeout_secs: u64,
    pub(in crate::dynamic_llm) capabilities: ModelCapabilities,
}

impl ResolvedLlmModel {
    pub(in crate::dynamic_llm) fn new(model: LlmProviderConfig) -> Result<Self, AppError> {
        a4n_llm_config::validate_llm_config(&llm_config_snapshot(&model))?;
        let timeout_secs = validated_timeout_secs(&model)?;
        let capabilities = model_capabilities(&model)?;
        Ok(Self {
            model,
            timeout_secs,
            capabilities,
        })
    }

    pub(in crate::dynamic_llm) fn model_name(&self) -> &str {
        &self.model.model
    }

    pub(in crate::dynamic_llm) fn concurrency_limit(
        &self,
    ) -> a4n_domain::task_queue::LlmConcurrencyLimit {
        let identity = serde_json::json!({
            "user": crate::runtime_config::current_user_id(),
            "config": self.model.id,
            "provider": self.model.provider.trim().to_ascii_lowercase(),
            "base_url": self.model.base_url.trim_end_matches('/'),
            "model": self.model.model,
            "credential": self.model.api_key,
        });
        a4n_domain::task_queue::LlmConcurrencyLimit {
            key: a4n_domain::source_text_hash(&identity.to_string()),
            max_concurrent: usize::try_from(self.model.max_concurrent)
                .expect("validated model concurrency"),
        }
    }

    pub(in crate::dynamic_llm) fn planner(&self) -> PlanningLlmProvider {
        PlanningLlmProvider {
            model_name: self.model.model.clone(),
            capabilities: self.capabilities.clone(),
        }
    }

    pub(in crate::dynamic_llm) fn sidecar_payload(
        self,
        request: ChatRequest,
    ) -> LlmGatewayPayload<LlmChatRequest> {
        LlmGatewayPayload {
            provider: self.provider_config(),
            request: request.into(),
        }
    }

    pub(in crate::dynamic_llm) fn probe_payload(
        self,
        request: ChatRequest,
    ) -> LlmProbePayload<LlmChatRequest> {
        LlmProbePayload {
            provider: self.provider_config(),
            request: request.into(),
        }
    }

    fn provider_config(self) -> LlmGatewayProviderConfig {
        LlmGatewayProviderConfig {
            provider: self.model.provider,
            base_url: self.model.base_url,
            model: self.model.model,
            api_key: self.model.api_key,
            timeout_secs: self.timeout_secs,
            extra_body_json: self.model.extra_body_json,
        }
    }
}

fn llm_config_snapshot(model: &LlmProviderConfig) -> LlmConfigSnapshot {
    LlmConfigSnapshot {
        name: model.name.clone(),
        provider: model.provider.clone(),
        base_url: model.base_url.clone(),
        api_key_encrypted: model.api_key.clone(),
        model: model.model.clone(),
        max_concurrent: model.max_concurrent,
        timeout_secs: model.timeout_secs,
        max_context_tokens: model.max_context_tokens,
        max_output_tokens: model.max_output_tokens,
        extra_body_json: model.extra_body_json.clone(),
    }
}

pub(in crate::dynamic_llm) struct PlanningLlmProvider {
    model_name: String,
    capabilities: ModelCapabilities,
}

#[async_trait]
impl LlmProvider for PlanningLlmProvider {
    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        Err(AppError::ConfigError(
            "PlanningLlmProvider 只用于上下文规划，不能执行 chat_stream".to_string(),
        ))
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }

    fn capabilities(&self) -> ModelCapabilities {
        self.capabilities.clone()
    }
}
