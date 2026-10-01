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
        llm_provider_config(a4n_db::config::llm_config_read_record(&model)).map_err(app_error)
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
        embedding_provider_config(a4n_db::config::embedding_config_read_record(&model))
            .map_err(app_error)
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
        image_connectivity_config(a4n_db::config::image_config_read_record(&model))
            .map_err(app_error)
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
                llm_summary(&a4n_db::config::llm_config_read_record(model)).map_err(app_error)
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
                image_summary(&a4n_db::config::image_config_read_record(model)).map_err(app_error)
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
                video_summary(&a4n_db::config::video_config_read_record(model)).map_err(app_error)
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
                embedding_summary(&a4n_db::config::embedding_config_read_record(model))
                    .map_err(app_error)
            })
            .collect()
    }

    async fn settings_export(
        &self,
        request: WorkerEnvelope<SettingsExportInput>,
    ) -> Result<String, WorkerFailure> {
        validate_admin_request(&request, CONFIG_SETTINGS_EXPORT_OPERATION, &self.database).await?;
        let rows = a4n_db::config::settings_all_config_rows(self.database.connection())
            .await
            .map_err(app_error)?;
        serde_json::to_string_pretty(&AllConfigsExport {
            version: SETTINGS_ALL_FORMAT_VERSION.to_string(),
            export_time: Utc::now().to_rfc3339(),
            llm_configs: rows.llm_configs,
            embedding_configs: rows.embedding_configs,
            image_configs: rows.image_configs,
            video_configs: rows.video_configs,
            prompts_modified: request.payload.prompts_modified,
            themes: rows.themes,
            system_settings: rows.system_settings,
        })
        .map_err(|error| WorkerFailure::unavailable(format!("配置导出序列化失败: {error}")))
    }

    async fn settings_prepare_import(
        &self,
        request: WorkerEnvelope<a4n_api_contract::config::ImportAllConfigsInput>,
    ) -> Result<SettingsPrepareImportResult, WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_SETTINGS_PREPARE_IMPORT_OPERATION,
            &self.database,
        )
        .await?;
        let mut incoming = parse_settings_export(&request.payload.settings_json)?;
        normalize_all_configs_export(&mut incoming).map_err(WorkerFailure::invalid_request)?;
        validate_all_configs_export(&incoming).map_err(WorkerFailure::invalid_request)?;
        let config_import = settings_import_input(&incoming, request.payload.overwrite);
        a4n_db::config::validate_settings_all_config_import_defaults(
            self.database.connection(),
            &config_import,
        )
        .await
        .map_err(settings_import_error)?;
        let settings_json = serde_json::to_string(&incoming)
            .map_err(|error| WorkerFailure::unavailable(format!("配置导入序列化失败: {error}")))?;
        Ok(SettingsPrepareImportResult {
            settings_json,
            prompts_modified: incoming.prompts_modified,
        })
    }

    async fn settings_import(
        &self,
        request: WorkerEnvelope<SettingsImportCommand>,
    ) -> Result<Vec<String>, WorkerFailure> {
        validate_admin_request(&request, CONFIG_SETTINGS_IMPORT_OPERATION, &self.database).await?;
        let mut incoming = parse_settings_export(&request.payload.settings_json)?;
        normalize_all_configs_export(&mut incoming).map_err(WorkerFailure::invalid_request)?;
        validate_all_configs_export(&incoming).map_err(WorkerFailure::invalid_request)?;
        if request.payload.prompt_imported > incoming.prompts_modified.len() {
            return Err(WorkerFailure::invalid_request(
                "导入提示词数量不能超过导出内容中的提示词数量",
            ));
        }
        let config_import = settings_import_input(&incoming, request.payload.overwrite);
        a4n_db::config::validate_settings_all_config_import_defaults(
            self.database.connection(),
            &config_import,
        )
        .await
        .map_err(settings_import_error)?;
        let counts =
            a4n_db::config::import_settings_all_configs(self.database.connection(), config_import)
                .await
                .map_err(settings_import_error)?;
        Ok(import_details_from_counts(
            counts,
            request.payload.prompt_imported,
        ))
    }

    async fn baseline_embedding(
        &self,
        request: WorkerEnvelope<EmptyInput>,
    ) -> Result<bool, WorkerFailure> {
        validate_service_request(&request, CONFIG_MUTATION_BASELINE_EMBEDDING_OPERATION)?;
        a4n_db::config::insert_baseline_embedding_config_if_empty(self.database.connection())
            .await
            .map_err(app_error)
    }

    async fn baseline_llm(
        &self,
        request: WorkerEnvelope<CreateLlmCommand>,
    ) -> Result<bool, WorkerFailure> {
        validate_service_request(&request, CONFIG_MUTATION_BASELINE_LLM_OPERATION)?;
        let input = llm_record_input(normalize_llm_input(request.payload.0).map_err(app_error)?);
        a4n_db::config::insert_baseline_llm_config_if_empty(self.database.connection(), input)
            .await
            .map_err(app_error)
    }

    async fn baseline_image(
        &self,
        request: WorkerEnvelope<CreateImageCommand>,
    ) -> Result<bool, WorkerFailure> {
        validate_service_request(&request, CONFIG_MUTATION_BASELINE_IMAGE_OPERATION)?;
        let (params, pricing) = image_params(request.payload.0).map_err(app_error)?;
        a4n_db::config::insert_baseline_image_config_if_empty(
            self.database.connection(),
            image_record_input(params, pricing).map_err(app_error)?,
        )
        .await
        .map_err(app_error)
    }

    async fn baseline_video(
        &self,
        request: WorkerEnvelope<CreateVideoCommand>,
    ) -> Result<bool, WorkerFailure> {
        validate_service_request(&request, CONFIG_MUTATION_BASELINE_VIDEO_OPERATION)?;
        let (params, pricing) = video_params(request.payload.0).map_err(app_error)?;
        a4n_db::config::insert_baseline_video_config_if_empty(
            self.database.connection(),
            video_record_input(params, pricing),
        )
        .await
        .map_err(app_error)
    }

    async fn create_llm(
        &self,
        request: WorkerEnvelope<CreateLlmCommand>,
    ) -> Result<LlmConfigSummary, WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_CREATE_LLM_OPERATION,
            &self.database,
        )
        .await?;
        let model = a4n_db::config::create_llm_config_record(
            self.database.connection(),
            llm_record_input(normalize_llm_input(request.payload.0).map_err(app_error)?),
        )
        .await
        .map_err(app_error)?;
        llm_summary(&a4n_db::config::llm_config_read_record(&model)).map_err(app_error)
    }

    async fn update_llm(
        &self,
        request: WorkerEnvelope<UpdateLlmCommand>,
    ) -> Result<LlmConfigSummary, WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_UPDATE_LLM_OPERATION,
            &self.database,
        )
        .await?;
        let existing = a4n_db::config::llm_config_by_id(
            self.database.connection(),
            positive_id(request.payload.config_id)?,
        )
        .await
        .map_err(app_error)?;
        let input =
            normalize_update_llm_input(&existing, request.payload.input).map_err(app_error)?;
        let model = a4n_db::config::update_llm_config_record(
            self.database.connection(),
            existing,
            llm_record_input(input),
        )
        .await
        .map_err(app_error)?;
        llm_summary(&a4n_db::config::llm_config_read_record(&model)).map_err(app_error)
    }

    async fn set_default_llm(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<(), WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_SET_DEFAULT_LLM_OPERATION,
            &self.database,
        )
        .await?;
        let config_id = positive_id(request.payload.config_id)?;
        let model = a4n_db::config::llm_config_by_id(self.database.connection(), config_id)
            .await
            .map_err(app_error)?;
        validate_llm_config(&llm_config_snapshot(&model)).map_err(app_error)?;
        a4n_db::config::set_default_provider_config(
            self.database.connection(),
            ProviderConfigKind::Llm,
            config_id,
        )
        .await
        .map_err(app_error)
    }

    async fn delete_llm(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<(), WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_DELETE_LLM_OPERATION,
            &self.database,
        )
        .await?;
        let config_id = positive_id(request.payload.config_id)?;
        let model = a4n_db::config::llm_config_by_id(self.database.connection(), config_id)
            .await
            .map_err(app_error)?;
        ensure_default_config_delete_allowed(" LLM 配置", model.is_default).map_err(app_error)?;
        a4n_db::config::delete_llm_config_record(self.database.connection(), config_id)
            .await
            .map_err(app_error)
    }

    async fn create_image(
        &self,
        request: WorkerEnvelope<CreateImageCommand>,
    ) -> Result<ImageConfigSummary, WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_CREATE_IMAGE_OPERATION,
            &self.database,
        )
        .await?;
        let (params, pricing) = image_params(request.payload.0).map_err(app_error)?;
        let model = a4n_db::config::create_image_config_record(
            self.database.connection(),
            image_record_input(params, pricing).map_err(app_error)?,
        )
        .await
        .map_err(app_error)?;
        image_summary(&a4n_db::config::image_config_read_record(&model)).map_err(app_error)
    }

    async fn update_image(
        &self,
        request: WorkerEnvelope<UpdateImageCommand>,
    ) -> Result<ImageConfigSummary, WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_UPDATE_IMAGE_OPERATION,
            &self.database,
        )
        .await?;
        let existing = a4n_db::config::image_config_by_id(
            self.database.connection(),
            positive_id(request.payload.config_id)?,
        )
        .await
        .map_err(app_error)?;
        let input = request.payload.input;
        let api_key_encrypted =
            merge_api_key_update(input.api_key_encrypted, existing.api_key_encrypted.clone());
        let pricing = pricing_json(input.pricing).map_err(app_error)?;
        let params = validate_image_config_params(a4n_domain::image::CreateImageConfigParams {
            name: input.name,
            backend: input.backend,
            base_url: input.base_url,
            api_key_encrypted,
            model: input.model,
            max_concurrent: input.max_concurrent,
            supported_sizes: input.supported_sizes,
        })
        .map_err(app_error)?;
        let model = a4n_db::config::update_image_config_record(
            self.database.connection(),
            existing,
            image_record_input(params, pricing).map_err(app_error)?,
        )
        .await
        .map_err(app_error)?;
        image_summary(&a4n_db::config::image_config_read_record(&model)).map_err(app_error)
    }

    async fn set_default_image(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<(), WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_SET_DEFAULT_IMAGE_OPERATION,
            &self.database,
        )
        .await?;
        let config_id = positive_id(request.payload.config_id)?;
        let model = a4n_db::config::image_config_by_id(self.database.connection(), config_id)
            .await
            .map_err(app_error)?;
        a4n_db::config::validate_image_config_model(&model).map_err(app_error)?;
        a4n_db::config::set_default_provider_config(
            self.database.connection(),
            ProviderConfigKind::Image,
            config_id,
        )
        .await
        .map_err(app_error)
    }

    async fn delete_image(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<(), WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_DELETE_IMAGE_OPERATION,
            &self.database,
        )
        .await?;
        let config_id = positive_id(request.payload.config_id)?;
        let model = a4n_db::config::image_config_by_id(self.database.connection(), config_id)
            .await
            .map_err(app_error)?;
        ensure_default_config_delete_allowed("图片配置", model.is_default).map_err(app_error)?;
        a4n_db::config::delete_image_config_record(self.database.connection(), config_id)
            .await
            .map_err(app_error)
    }

    async fn create_video(
        &self,
        request: WorkerEnvelope<CreateVideoCommand>,
    ) -> Result<VideoConfigSummary, WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_CREATE_VIDEO_OPERATION,
            &self.database,
        )
        .await?;
        let (params, pricing) = video_params(request.payload.0).map_err(app_error)?;
        let model = a4n_db::config::create_video_config_record(
            self.database.connection(),
            video_record_input(params, pricing),
        )
        .await
        .map_err(app_error)?;
        video_summary(&a4n_db::config::video_config_read_record(&model)).map_err(app_error)
    }

    async fn update_video(
        &self,
        request: WorkerEnvelope<UpdateVideoCommand>,
    ) -> Result<VideoConfigSummary, WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_UPDATE_VIDEO_OPERATION,
            &self.database,
        )
        .await?;
        let existing = a4n_db::config::video_config_by_id(
            self.database.connection(),
            positive_id(request.payload.config_id)?,
        )
        .await
        .map_err(app_error)?;
        let input = request.payload.input;
        let params = validate_video_config_params(VideoConfigParams {
            name: input.name,
            backend: input.backend,
            base_url: input.base_url,
            api_key_encrypted: merge_api_key_update(
                input.api_key_encrypted,
                existing.api_key_encrypted.clone(),
            ),
            model: input.model,
            max_concurrent: input.max_concurrent,
            default_width: input.default_width,
            default_height: input.default_height,
            default_duration_secs: input.default_duration_secs,
            min_duration_secs: input.min_duration_secs,
            max_duration_secs: input.max_duration_secs,
            duration_mode: input.duration_mode.to_string(),
            supported_durations_json: input.supported_durations_json,
            default_fps: input.default_fps,
        })
        .map_err(|error| app_error(a4n_error::AppError::InvalidArgument(error)))?;
        let model = a4n_db::config::update_video_config_record(
            self.database.connection(),
            existing,
            video_record_input(params, pricing_json(input.pricing).map_err(app_error)?),
        )
        .await
        .map_err(app_error)?;
        video_summary(&a4n_db::config::video_config_read_record(&model)).map_err(app_error)
    }

    async fn set_default_video(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<(), WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_SET_DEFAULT_VIDEO_OPERATION,
            &self.database,
        )
        .await?;
        let config_id = positive_id(request.payload.config_id)?;
        let model = a4n_db::config::video_config_by_id(self.database.connection(), config_id)
            .await
            .map_err(app_error)?;
        validate_video_model(&model).map_err(app_error)?;
        a4n_db::config::set_default_provider_config(
            self.database.connection(),
            ProviderConfigKind::Video,
            config_id,
        )
        .await
        .map_err(app_error)
    }

    async fn delete_video(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<(), WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_DELETE_VIDEO_OPERATION,
            &self.database,
        )
        .await?;
        let config_id = positive_id(request.payload.config_id)?;
        let model = a4n_db::config::video_config_by_id(self.database.connection(), config_id)
            .await
            .map_err(app_error)?;
        ensure_default_config_delete_allowed("视频配置", model.is_default).map_err(app_error)?;
        let active = a4n_db::config::active_generated_video_ids_for_config(
            self.database.connection(),
            config_id,
        )
        .await
        .map_err(app_error)?;
        if !active.is_empty() {
            return Err(WorkerFailure::invalid_request(format!(
                "视频配置 #{config_id} 仍有 {} 个进行中的视频任务，不能删除",
                active.len()
            )));
        }
        a4n_db::config::delete_video_config_record(self.database.connection(), config_id)
            .await
            .map_err(app_error)
    }

    async fn create_embedding(
        &self,
        request: WorkerEnvelope<CreateEmbeddingCommand>,
    ) -> Result<EmbeddingConfigSummary, WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_CREATE_EMBEDDING_OPERATION,
            &self.database,
        )
        .await?;
        let (params, pricing) = embedding_params(request.payload.0).map_err(app_error)?;
        let model = a4n_db::config::create_embedding_config_record(
            self.database.connection(),
            embedding_record_input(params, pricing),
        )
        .await
        .map_err(app_error)?;
        embedding_summary(&a4n_db::config::embedding_config_read_record(&model)).map_err(app_error)
    }

    async fn update_embedding(
        &self,
        request: WorkerEnvelope<UpdateEmbeddingCommand>,
    ) -> Result<EmbeddingConfigSummary, WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_UPDATE_EMBEDDING_OPERATION,
            &self.database,
        )
        .await?;
        let existing = a4n_db::config::embedding_config_by_id(
            self.database.connection(),
            positive_id(request.payload.config_id)?,
        )
        .await
        .map_err(app_error)?;
        let input = request.payload.input;
        let params = validate_embedding_config_params(CreateEmbeddingConfigParams {
            name: input.name,
            backend: input.backend,
            base_url: input.base_url,
            api_key_encrypted: merge_api_key_update(
                input.api_key_encrypted,
                existing.api_key_encrypted.clone(),
            ),
            model: input.model,
            dimension: input.dimension,
            batch_size: input.batch_size,
            local_model_path: input.local_model_path,
        })
        .map_err(app_error)?;
        let model = a4n_db::config::update_embedding_config_record(
            self.database.connection(),
            existing,
            embedding_record_input(params, pricing_json(input.pricing).map_err(app_error)?),
        )
        .await
        .map_err(app_error)?;
        embedding_summary(&a4n_db::config::embedding_config_read_record(&model)).map_err(app_error)
    }

    async fn set_default_embedding(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<(), WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_SET_DEFAULT_EMBEDDING_OPERATION,
            &self.database,
        )
        .await?;
        let config_id = positive_id(request.payload.config_id)?;
        let model = a4n_db::config::embedding_config_by_id(self.database.connection(), config_id)
            .await
            .map_err(app_error)?;
        a4n_db::config::validate_embedding_config_model(&model).map_err(app_error)?;
        a4n_db::config::set_default_provider_config(
            self.database.connection(),
            ProviderConfigKind::Embedding,
            config_id,
        )
        .await
        .map_err(app_error)
    }

    async fn delete_embedding(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<(), WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_DELETE_EMBEDDING_OPERATION,
            &self.database,
        )
        .await?;
        let config_id = positive_id(request.payload.config_id)?;
        let model = a4n_db::config::embedding_config_by_id(self.database.connection(), config_id)
            .await
            .map_err(app_error)?;
        ensure_default_config_delete_allowed(" Embedding 配置", model.is_default)
            .map_err(app_error)?;
        a4n_db::config::delete_embedding_config_record(self.database.connection(), config_id)
            .await
            .map_err(app_error)
    }

    fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
    }
}
