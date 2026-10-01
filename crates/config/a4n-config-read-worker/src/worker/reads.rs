impl WorkerState {
    async fn llm_exists(&self, request: WorkerEnvelope<EmptyInput>) -> Result<bool, WorkerFailure> {
        validate_service_request(&request, CONFIG_READ_LLM_EXISTS_OPERATION)?;
        a4n_db::config::llm_configs_exist(self.database.connection())
            .await
            .map_err(app_error)
    }

    async fn image_exists(
        &self,
        request: WorkerEnvelope<EmptyInput>,
    ) -> Result<bool, WorkerFailure> {
        validate_service_request(&request, CONFIG_READ_IMAGE_EXISTS_OPERATION)?;
        a4n_db::config::image_configs_exist(self.database.connection())
            .await
            .map_err(app_error)
    }

    async fn video_exists(
        &self,
        request: WorkerEnvelope<EmptyInput>,
    ) -> Result<bool, WorkerFailure> {
        validate_service_request(&request, CONFIG_READ_VIDEO_EXISTS_OPERATION)?;
        a4n_db::config::video_configs_exist(self.database.connection())
            .await
            .map_err(app_error)
    }

    async fn saved_api_key(
        &self,
        request: WorkerEnvelope<SavedApiKeyInput>,
    ) -> Result<Option<String>, WorkerFailure> {
        validate_user_request(&request, CONFIG_READ_SAVED_API_KEY_OPERATION)?;
        require_admin(&self.database, user_id(&request)?).await?;
        let config_id = positive_id(request.payload.config_id)?;
        let value = match request.payload.kind {
            ConfigProviderKindWire::Llm => {
                a4n_db::config::llm_config_by_id(self.database.connection(), config_id)
                    .await
                    .map_err(app_error)?
                    .api_key_encrypted
            }
            ConfigProviderKindWire::Image => {
                a4n_db::config::image_config_by_id(self.database.connection(), config_id)
                    .await
                    .map_err(app_error)?
                    .api_key_encrypted
            }
            ConfigProviderKindWire::Video => {
                a4n_db::config::video_config_by_id(self.database.connection(), config_id)
                    .await
                    .map_err(app_error)?
                    .api_key_encrypted
            }
        };
        Ok(value
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()))
    }

    async fn llm_connectivity(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<LlmProviderConfig, WorkerFailure> {
        validate_user_request(&request, CONFIG_READ_LLM_CONNECTIVITY_OPERATION)?;
        require_admin(&self.database, user_id(&request)?).await?;
        let model = a4n_db::config::llm_config_by_id(
            self.database.connection(),
            positive_id(request.payload.config_id)?,
        )
        .await
        .map_err(app_error)?;
        llm_provider_config(llm_config_read_record(&model)).map_err(app_error)
    }

    async fn embedding_connectivity(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<EmbeddingProviderConfig, WorkerFailure> {
        validate_user_request(&request, CONFIG_READ_EMBEDDING_CONNECTIVITY_OPERATION)?;
        require_admin(&self.database, user_id(&request)?).await?;
        let model = a4n_db::config::embedding_config_by_id(
            self.database.connection(),
            positive_id(request.payload.config_id)?,
        )
        .await
        .map_err(app_error)?;
        embedding_provider_config(embedding_config_read_record(&model)).map_err(app_error)
    }

    async fn image_connectivity(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<ImageConfigForConnectivityTest, WorkerFailure> {
        validate_user_request(&request, CONFIG_READ_IMAGE_CONNECTIVITY_OPERATION)?;
        require_admin(&self.database, user_id(&request)?).await?;
        let model = a4n_db::config::image_config_by_id(
            self.database.connection(),
            positive_id(request.payload.config_id)?,
        )
        .await
        .map_err(app_error)?;
        image_connectivity_config(image_config_read_record(&model)).map_err(app_error)
    }

    async fn llm_list(
        &self,
        request: WorkerEnvelope<EmptyInput>,
    ) -> Result<Vec<LlmConfigSummary>, WorkerFailure> {
        validate_service_request(&request, CONFIG_READ_LLM_LIST_OPERATION)?;
        a4n_db::config::list_llm_config_records(self.database.connection())
            .await
            .map_err(app_error)?
            .iter()
            .map(|model| {
                llm_summary(&llm_config_read_record(model)).map_err(app_error)
            })
            .collect()
    }

    async fn image_list(
        &self,
        request: WorkerEnvelope<EmptyInput>,
    ) -> Result<Vec<ImageConfigSummary>, WorkerFailure> {
        validate_service_request(&request, CONFIG_READ_IMAGE_LIST_OPERATION)?;
        a4n_db::config::list_image_config_records(self.database.connection())
            .await
            .map_err(app_error)?
            .iter()
            .map(|model| {
                image_summary(&image_config_read_record(model)).map_err(app_error)
            })
            .collect()
    }

    async fn video_list(
        &self,
        request: WorkerEnvelope<EmptyInput>,
    ) -> Result<Vec<VideoConfigSummary>, WorkerFailure> {
        validate_service_request(&request, CONFIG_READ_VIDEO_LIST_OPERATION)?;
        a4n_db::config::list_video_config_records(self.database.connection())
            .await
            .map_err(app_error)?
            .iter()
            .map(|model| {
                video_summary(&video_config_read_record(model)).map_err(app_error)
            })
            .collect()
    }

    async fn embedding_list(
        &self,
        request: WorkerEnvelope<EmptyInput>,
    ) -> Result<Vec<EmbeddingConfigSummary>, WorkerFailure> {
        validate_service_request(&request, CONFIG_READ_EMBEDDING_LIST_OPERATION)?;
        a4n_db::config::list_embedding_configs(self.database.connection())
            .await
            .map_err(app_error)?
            .iter()
            .map(|model| {
                embedding_summary(&embedding_config_read_record(model)).map_err(app_error)
            })
            .collect()
    }
}
