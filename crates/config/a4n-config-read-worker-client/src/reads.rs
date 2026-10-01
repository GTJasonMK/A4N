#[async_trait::async_trait]
impl ConfigReadApplicationPort for ConfigReadWorkerClient {
    async fn llm_configs_exist(&self) -> Result<bool, AppError> {
        self.command(
            Self::service(),
            CONFIG_READ_LLM_EXISTS_OPERATION,
            EmptyInput {},
        )
        .await
    }

    async fn image_configs_exist(&self) -> Result<bool, AppError> {
        self.command(
            Self::service(),
            CONFIG_READ_IMAGE_EXISTS_OPERATION,
            EmptyInput {},
        )
        .await
    }

    async fn video_configs_exist(&self) -> Result<bool, AppError> {
        self.command(
            Self::service(),
            CONFIG_READ_VIDEO_EXISTS_OPERATION,
            EmptyInput {},
        )
        .await
    }

    async fn saved_api_key(
        &self,
        caller_user_id: i64,
        kind: ConfigProviderKind,
        config_id: i64,
    ) -> Result<Option<String>, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_READ_SAVED_API_KEY_OPERATION,
            SavedApiKeyInput {
                kind: Self::kind(kind),
                config_id,
            },
        )
        .await
    }

    async fn llm_config_for_connectivity_test(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<LlmProviderConfig, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_READ_LLM_CONNECTIVITY_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }

    async fn embedding_config_for_connectivity_test(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<EmbeddingProviderConfig, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_READ_EMBEDDING_CONNECTIVITY_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }

    async fn image_config_for_connectivity_test(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<ImageConfigForConnectivityTest, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_READ_IMAGE_CONNECTIVITY_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }

    async fn list_llm(&self) -> Result<Vec<LlmConfigSummary>, AppError> {
        self.command(
            Self::service(),
            CONFIG_READ_LLM_LIST_OPERATION,
            EmptyInput {},
        )
        .await
    }

    async fn list_image(&self) -> Result<Vec<ImageConfigSummary>, AppError> {
        self.command(
            Self::service(),
            CONFIG_READ_IMAGE_LIST_OPERATION,
            EmptyInput {},
        )
        .await
    }

    async fn list_video(&self) -> Result<Vec<VideoConfigSummary>, AppError> {
        self.command(
            Self::service(),
            CONFIG_READ_VIDEO_LIST_OPERATION,
            EmptyInput {},
        )
        .await
    }

    async fn list_embedding(&self) -> Result<Vec<EmbeddingConfigSummary>, AppError> {
        self.command(
            Self::service(),
            CONFIG_READ_EMBEDDING_LIST_OPERATION,
            EmptyInput {},
        )
        .await
    }
}
