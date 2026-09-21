//! 配置写入合同实现。

use super::*;

#[async_trait::async_trait]
impl a4n_api_contract::config::ConfigMutationApplicationPort for OrchestratorContext {
    async fn insert_baseline_embedding_if_empty(&self) -> Result<bool, AppError> {
        OrchestratorContext::insert_baseline_embedding_config_if_empty(self).await
    }

    async fn insert_baseline_llm_if_empty(&self, input: CreateLlmInput) -> Result<bool, AppError> {
        OrchestratorContext::insert_baseline_llm_config_if_empty(
            self,
            normalize_llm_config_input(input)?,
        )
        .await
    }

    async fn insert_baseline_image_if_empty(
        &self,
        input: ImageConfigInput,
    ) -> Result<bool, AppError> {
        let (params, _) = image_config_params(input)?;
        OrchestratorContext::insert_baseline_image_config_if_empty(self, params).await
    }

    async fn insert_baseline_video_if_empty(
        &self,
        input: VideoConfigInput,
    ) -> Result<bool, AppError> {
        let (params, _) = video_config_params(input)?;
        OrchestratorContext::insert_baseline_video_config_if_empty(self, params).await
    }

    async fn create_llm(
        &self,
        caller_user_id: i64,
        input: CreateLlmInput,
    ) -> Result<LlmConfigSummary, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        llm_config_summary(&self.create_llm_config_validated(input).await?)
    }

    async fn update_llm(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateLlmInput,
    ) -> Result<LlmConfigSummary, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        llm_config_summary(&self.update_llm_config_validated(config_id, input).await?)
    }

    async fn set_default_llm(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.require_settings_admin(caller_user_id).await?;
        self.set_default_llm_config_validated(config_id).await
    }

    async fn delete_llm(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.require_settings_admin(caller_user_id).await?;
        self.delete_llm_config_if_allowed(config_id).await
    }

    async fn create_image(
        &self,
        caller_user_id: i64,
        input: ImageConfigInput,
    ) -> Result<ImageConfigSummary, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        image_config_summary(&self.create_image_config_validated(input).await?)
    }

    async fn update_image(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateImageConfigInput,
    ) -> Result<ImageConfigSummary, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        image_config_summary(&self.update_image_config_validated(config_id, input).await?)
    }

    async fn set_default_image(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.require_settings_admin(caller_user_id).await?;
        self.set_default_image_config_validated(config_id).await
    }

    async fn delete_image(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.require_settings_admin(caller_user_id).await?;
        self.delete_image_config_if_allowed(config_id).await
    }

    async fn create_video(
        &self,
        caller_user_id: i64,
        input: VideoConfigInput,
    ) -> Result<VideoConfigSummary, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        video_config_summary(&self.create_video_config_validated(input).await?)
    }

    async fn update_video(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateVideoConfigInput,
    ) -> Result<VideoConfigSummary, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        video_config_summary(&self.update_video_config_validated(config_id, input).await?)
    }

    async fn set_default_video(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.require_settings_admin(caller_user_id).await?;
        self.set_default_video_config_validated(config_id).await
    }

    async fn delete_video(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.require_settings_admin(caller_user_id).await?;
        self.delete_video_config_if_allowed(config_id).await
    }

    async fn create_embedding(
        &self,
        caller_user_id: i64,
        input: CreateEmbeddingInput,
    ) -> Result<EmbeddingConfigSummary, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        embedding_config_summary(&self.create_embedding_config_validated(input).await?)
    }

    async fn update_embedding(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateEmbeddingInput,
    ) -> Result<EmbeddingConfigSummary, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        embedding_config_summary(
            &self
                .update_embedding_config_validated(config_id, input)
                .await?,
        )
    }

    async fn set_default_embedding(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.require_settings_admin(caller_user_id).await?;
        self.set_default_embedding_config_validated(config_id).await
    }

    async fn delete_embedding(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.require_settings_admin(caller_user_id).await?;
        self.delete_embedding_config_if_allowed(config_id).await
    }
}
