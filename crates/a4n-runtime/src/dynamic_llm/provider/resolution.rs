use std::sync::Arc;

use crate::provider_runtime::DynamicProviderFacade;
use a4n_error::AppError;
use a4n_llm_client::LlmClient;
use a4n_llm_provider_contract::provider::ModelCapabilities;
use a4n_provider_contract::LlmProviderConfig;

use super::super::contracts::{DynamicLlmProvider, ResolvedLlmModel};
use crate::runtime_config;

impl DynamicLlmProvider {
    pub(crate) async fn agent_protocol_for_config_id(
        &self,
        config_id: Option<i64>,
    ) -> Result<String, AppError> {
        let model = self.resolve_model_by_id(config_id).await?;
        if !a4n_domain::agent::valid_agent_protocol(&model.agent_protocol) {
            return Err(AppError::ConfigError(format!(
                "LLM 配置 {} 使用不支持的 agent_protocol: {}",
                model.name, model.agent_protocol
            )));
        }
        Ok(model.agent_protocol)
    }

    pub(crate) async fn capabilities_for_config_id(
        &self,
        config_id: Option<i64>,
    ) -> Result<ModelCapabilities, AppError> {
        let model = self.resolve_model_by_id(config_id).await?;
        super::super::config::model_capabilities(&model)
    }

    /// 使用动态 provider 窄 facade 构造（不持有完整编排上下文）。
    pub(crate) fn new(facade: DynamicProviderFacade) -> Self {
        Self {
            facade,
            sidecar_client: None,
        }
    }

    pub(crate) fn new_with_sidecar_client(
        facade: DynamicProviderFacade,
        sidecar_client: Arc<dyn LlmClient>,
    ) -> Self {
        Self {
            facade,
            sidecar_client: Some(sidecar_client),
        }
    }

    pub(super) async fn resolve_model_by_id(
        &self,
        config_id: Option<i64>,
    ) -> Result<LlmProviderConfig, AppError> {
        if let Some(snapshot_model) = runtime_config::current_llm_provider_config_for_id(config_id)
        {
            return Ok(snapshot_model);
        }
        if config_id.is_none() && runtime_config::uses_system_default_llm_template() {
            return self.resolve_system_default_template_with_user_key().await;
        }
        if let Some(id) = config_id
            && runtime_config::current_llm_snapshot_scope_present()
        {
            return Err(AppError::ConfigError(format!(
                "runtime_config_snapshot.llm_configs 不包含 LLM 配置 #{id}"
            )));
        }
        if runtime_config::current_user_id().is_some() {
            return Err(AppError::ConfigError(match config_id {
                Some(id) => format!(
                    "远程用户请求不能直接使用服务器 LLM 配置 #{id}；请在用户配置页使用个人 LLM 配置快照"
                ),
                None => "使用系统默认 LLM 需要先在用户配置页填写并保存你的个人 API Key".to_string(),
            }));
        }

        self.facade.resolve_llm_config(config_id).await
    }

    async fn resolve_system_default_template_with_user_key(
        &self,
    ) -> Result<LlmProviderConfig, AppError> {
        let api_key = runtime_config::current_system_llm_api_key().ok_or_else(|| {
            AppError::ConfigError(
                "使用系统默认 LLM 需要先在用户配置页填写并保存你的个人 API Key".to_string(),
            )
        })?;
        let mut model = self.facade.resolve_llm_config(None).await?;
        model.api_key = Some(api_key);
        Ok(model)
    }

    pub(in crate::dynamic_llm) async fn resolve_runtime_model_by_id(
        &self,
        config_id: Option<i64>,
    ) -> Result<ResolvedLlmModel, AppError> {
        let model = self.resolve_model_by_id(config_id).await?;
        ResolvedLlmModel::new(model)
    }
}
