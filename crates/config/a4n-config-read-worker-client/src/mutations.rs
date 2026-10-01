#[async_trait::async_trait]
impl ConfigMutationApplicationPort for ConfigReadWorkerClient {
    async fn insert_baseline_embedding_if_empty(&self) -> Result<bool, AppError> {
        self.command(
            Self::service(),
            CONFIG_MUTATION_BASELINE_EMBEDDING_OPERATION,
            EmptyInput {},
        )
        .await
    }

    async fn insert_baseline_llm_if_empty(&self, input: CreateLlmInput) -> Result<bool, AppError> {
        self.command(
            Self::service(),
            CONFIG_MUTATION_BASELINE_LLM_OPERATION,
            CreateLlmCommand(input),
        )
        .await
    }

    async fn insert_baseline_image_if_empty(
        &self,
        input: ImageConfigInput,
    ) -> Result<bool, AppError> {
        self.command(
            Self::service(),
            CONFIG_MUTATION_BASELINE_IMAGE_OPERATION,
            CreateImageCommand(input),
        )
        .await
    }

    async fn insert_baseline_video_if_empty(
        &self,
        input: VideoConfigInput,
    ) -> Result<bool, AppError> {
        self.command(
            Self::service(),
            CONFIG_MUTATION_BASELINE_VIDEO_OPERATION,
            CreateVideoCommand(input),
        )
        .await
    }

    async fn create_llm(
        &self,
        caller_user_id: i64,
        input: CreateLlmInput,
    ) -> Result<LlmConfigSummary, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_CREATE_LLM_OPERATION,
            CreateLlmCommand(input),
        )
        .await
    }

    async fn update_llm(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateLlmInput,
    ) -> Result<LlmConfigSummary, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_UPDATE_LLM_OPERATION,
            UpdateLlmCommand { config_id, input },
        )
        .await
    }

    async fn set_default_llm(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_SET_DEFAULT_LLM_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }

    async fn delete_llm(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_DELETE_LLM_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }

    async fn create_image(
        &self,
        caller_user_id: i64,
        input: ImageConfigInput,
    ) -> Result<ImageConfigSummary, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_CREATE_IMAGE_OPERATION,
            CreateImageCommand(input),
        )
        .await
    }

    async fn update_image(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateImageConfigInput,
    ) -> Result<ImageConfigSummary, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_UPDATE_IMAGE_OPERATION,
            UpdateImageCommand { config_id, input },
        )
        .await
    }

    async fn set_default_image(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_SET_DEFAULT_IMAGE_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }

    async fn delete_image(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_DELETE_IMAGE_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }

    async fn create_video(
        &self,
        caller_user_id: i64,
        input: VideoConfigInput,
    ) -> Result<VideoConfigSummary, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_CREATE_VIDEO_OPERATION,
            CreateVideoCommand(input),
        )
        .await
    }

    async fn update_video(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateVideoConfigInput,
    ) -> Result<VideoConfigSummary, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_UPDATE_VIDEO_OPERATION,
            UpdateVideoCommand { config_id, input },
        )
        .await
    }

    async fn set_default_video(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_SET_DEFAULT_VIDEO_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }

    async fn delete_video(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_DELETE_VIDEO_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }

    async fn create_embedding(
        &self,
        caller_user_id: i64,
        input: CreateEmbeddingInput,
    ) -> Result<EmbeddingConfigSummary, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_CREATE_EMBEDDING_OPERATION,
            CreateEmbeddingCommand(input),
        )
        .await
    }

    async fn update_embedding(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateEmbeddingInput,
    ) -> Result<EmbeddingConfigSummary, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_UPDATE_EMBEDDING_OPERATION,
            UpdateEmbeddingCommand { config_id, input },
        )
        .await
    }

    async fn set_default_embedding(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_SET_DEFAULT_EMBEDDING_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }

    async fn delete_embedding(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_DELETE_EMBEDDING_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }
}
