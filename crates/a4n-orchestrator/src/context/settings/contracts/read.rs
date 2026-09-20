//! 配置读取合同实现。

use super::*;

#[async_trait::async_trait]
impl a4n_api_contract::config::ConfigReadApplicationPort for OrchestratorContext {
    async fn llm_configs_exist(&self) -> Result<bool, AppError> {
        OrchestratorContext::llm_configs_exist(self).await
    }

    async fn image_configs_exist(&self) -> Result<bool, AppError> {
        OrchestratorContext::image_configs_exist(self).await
    }

    async fn video_configs_exist(&self) -> Result<bool, AppError> {
        OrchestratorContext::video_configs_exist(self).await
    }

    async fn saved_api_key(
        &self,
        caller_user_id: i64,
        kind: ConfigProviderKind,
        config_id: i64,
    ) -> Result<Option<String>, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        if config_id <= 0 {
            return Err(AppError::InvalidArgument(
                "配置 ID 必须为正整数".to_string(),
            ));
        }
        let api_key = match kind {
            ConfigProviderKind::Llm => self.llm_config_by_id(config_id).await?.api_key_encrypted,
            ConfigProviderKind::Image => {
                self.image_config_by_id(config_id).await?.api_key_encrypted
            }
            ConfigProviderKind::Video => {
                self.video_config_by_id(config_id).await?.api_key_encrypted
            }
        };
        Ok(api_key.and_then(|value| {
            let value = value.trim().to_string();
            (!value.is_empty()).then_some(value)
        }))
    }

    async fn llm_config_for_connectivity_test(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<LlmProviderConfig, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        validate_positive_config_id(config_id)?;
        llm_provider_config_from_model(self.llm_config_by_id(config_id).await?)
    }

    async fn embedding_config_for_connectivity_test(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<EmbeddingProviderConfig, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        validate_positive_config_id(config_id)?;
        embedding_provider_config_from_model(self.embedding_config_by_id(config_id).await?)
    }

    async fn image_config_for_connectivity_test(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<ImageConfigForConnectivityTest, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        validate_positive_config_id(config_id)?;
        image_config_for_connectivity_test_from_model(self.image_config_by_id(config_id).await?)
    }

    async fn list_llm(&self) -> Result<Vec<LlmConfigSummary>, AppError> {
        self.list_llm_config_records()
            .await?
            .iter()
            .map(llm_config_summary)
            .collect()
    }

    async fn list_image(&self) -> Result<Vec<ImageConfigSummary>, AppError> {
        self.list_image_config_records()
            .await?
            .iter()
            .map(image_config_summary)
            .collect()
    }

    async fn list_video(&self) -> Result<Vec<VideoConfigSummary>, AppError> {
        self.list_video_config_records()
            .await?
            .iter()
            .map(video_config_summary)
            .collect()
    }

    async fn list_embedding(&self) -> Result<Vec<EmbeddingConfigSummary>, AppError> {
        self.list_embedding_configs()
            .await?
            .iter()
            .map(embedding_config_summary)
            .collect()
    }
}
