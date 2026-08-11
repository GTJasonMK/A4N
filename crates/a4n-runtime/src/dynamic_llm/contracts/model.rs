use crate::dynamic_llm::config::{estimate_llm_tokens, model_capabilities, validated_timeout_secs};
use a4n_core::{error::AppError, models::ai::llm_config};
use a4n_llm::gateway::{
    LlmGatewayPayload as LlmChatPayload, LlmGatewayProviderConfig, LlmProbePayload,
};
use a4n_llm::provider::{ChatChunk, ChatRequest, LlmProvider, ModelCapabilities};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

#[derive(Debug, Clone)]
pub(in crate::dynamic_llm) struct ResolvedLlmModel {
    pub(in crate::dynamic_llm) model: llm_config::Model,
    pub(in crate::dynamic_llm) timeout_secs: u64,
    pub(in crate::dynamic_llm) capabilities: ModelCapabilities,
}

impl ResolvedLlmModel {
    pub(in crate::dynamic_llm) fn new(model: llm_config::Model) -> Result<Self, AppError> {
        a4n_llm::config::validate_llm_config_model(&model)?;
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

    pub(in crate::dynamic_llm) fn planner(&self) -> PlanningLlmProvider {
        PlanningLlmProvider {
            model_name: self.model.model.clone(),
            capabilities: self.capabilities.clone(),
        }
    }

    pub(in crate::dynamic_llm) fn sidecar_payload(self, request: ChatRequest) -> LlmChatPayload {
        LlmChatPayload {
            provider: self.provider_config(),
            request,
        }
    }

    pub(in crate::dynamic_llm) fn probe_payload(self, request: ChatRequest) -> LlmProbePayload {
        LlmProbePayload {
            provider: self.provider_config(),
            request: Some(request),
        }
    }

    fn provider_config(self) -> LlmGatewayProviderConfig {
        LlmGatewayProviderConfig {
            provider: self.model.provider,
            base_url: self.model.base_url,
            model: self.model.model,
            api_key: self.model.api_key_encrypted,
            timeout_secs: self.timeout_secs,
            extra_body_json: self.model.extra_body_json,
        }
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

    async fn test_connection(&self) -> Result<bool, AppError> {
        Err(AppError::ConfigError(
            "PlanningLlmProvider 只用于上下文规划，不能执行连接测试".to_string(),
        ))
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }

    fn capabilities(&self) -> ModelCapabilities {
        self.capabilities.clone()
    }

    fn count_tokens(&self, text: &str) -> usize {
        estimate_llm_tokens(text)
    }
}
