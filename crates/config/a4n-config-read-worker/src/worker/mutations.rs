use a4n_api_contract::config::{CreateEmbeddingInput, ImageConfigInput, VideoConfigInput};
use a4n_config::{
    image_sizes_to_json, validate_embedding_config_params, validate_image_config_params,
};
use a4n_config_read_worker_contract::{
    CONFIG_MUTATION_BASELINE_EMBEDDING_OPERATION, CONFIG_MUTATION_BASELINE_IMAGE_OPERATION,
    CONFIG_MUTATION_BASELINE_VIDEO_OPERATION, CONFIG_MUTATION_CREATE_EMBEDDING_OPERATION,
    CONFIG_MUTATION_CREATE_IMAGE_OPERATION, CONFIG_MUTATION_CREATE_VIDEO_OPERATION,
    CONFIG_MUTATION_DELETE_EMBEDDING_OPERATION, CONFIG_MUTATION_DELETE_IMAGE_OPERATION,
    CONFIG_MUTATION_DELETE_VIDEO_OPERATION, CONFIG_MUTATION_SET_DEFAULT_EMBEDDING_OPERATION,
    CONFIG_MUTATION_SET_DEFAULT_IMAGE_OPERATION, CONFIG_MUTATION_SET_DEFAULT_VIDEO_OPERATION,
    CONFIG_MUTATION_UPDATE_EMBEDDING_OPERATION, CONFIG_MUTATION_UPDATE_IMAGE_OPERATION,
    CONFIG_MUTATION_UPDATE_VIDEO_OPERATION,
};
use a4n_db::config::{
    EmbeddingConfigRecordInput, ImageConfigRecordInput, VideoConfigRecordInput,
};
use a4n_domain::embedding::CreateEmbeddingConfigParams;
use a4n_video::{VideoConfigParams, validate_video_config_params};

impl WorkerState {
    async fn baseline_embedding(
        &self,
        request: WorkerEnvelope<EmptyInput>,
    ) -> Result<bool, WorkerFailure> {
        validate_service_request(&request, CONFIG_MUTATION_BASELINE_EMBEDDING_OPERATION)?;
        a4n_db::config::insert_baseline_embedding_config_if_empty(self.database.connection())
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
        image_summary(&model).map_err(app_error)
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
        image_summary(&model).map_err(app_error)
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
        video_summary(&model).map_err(app_error)
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
        video_summary(&model).map_err(app_error)
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
        embedding_summary(&model).map_err(app_error)
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
        embedding_summary(&model).map_err(app_error)
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
}

fn image_params(
    input: ImageConfigInput,
) -> Result<(a4n_domain::image::CreateImageConfigParams, Value), a4n_error::AppError> {
    let pricing = pricing_json(input.pricing)?;
    let params = validate_image_config_params(a4n_domain::image::CreateImageConfigParams {
        name: input.name,
        backend: input.backend,
        base_url: input.base_url,
        api_key_encrypted: input.api_key_encrypted,
        model: input.model,
        max_concurrent: input.max_concurrent,
        supported_sizes: input.supported_sizes,
    })?;
    Ok((params, pricing))
}

fn image_record_input(
    params: a4n_domain::image::CreateImageConfigParams,
    pricing_json: Value,
) -> Result<ImageConfigRecordInput, a4n_error::AppError> {
    Ok(ImageConfigRecordInput {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted: params.api_key_encrypted,
        model: params.model,
        max_concurrent: params.max_concurrent,
        supported_sizes_json: image_sizes_to_json(&params.supported_sizes)?,
        pricing_json,
    })
}

fn video_params(
    input: VideoConfigInput,
) -> Result<(VideoConfigParams, Value), a4n_error::AppError> {
    let pricing = pricing_json(input.pricing)?;
    let params = validate_video_config_params(VideoConfigParams {
        name: input.name,
        backend: input.backend,
        base_url: input.base_url,
        api_key_encrypted: input.api_key_encrypted,
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
    .map_err(a4n_error::AppError::InvalidArgument)?;
    Ok((params, pricing))
}

fn video_record_input(params: VideoConfigParams, pricing_json: Value) -> VideoConfigRecordInput {
    VideoConfigRecordInput {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted: params.api_key_encrypted,
        model: params.model,
        max_concurrent: params.max_concurrent,
        default_width: params.default_width,
        default_height: params.default_height,
        default_duration_secs: params.default_duration_secs,
        min_duration_secs: params.min_duration_secs,
        max_duration_secs: params.max_duration_secs,
        duration_mode: params.duration_mode,
        supported_durations_json: params.supported_durations_json,
        default_fps: params.default_fps,
        pricing_json,
    }
}

fn validate_video_model(
    model: &a4n_db_models::models::ai::video_gen_config::Model,
) -> Result<(), a4n_error::AppError> {
    validate_video_config_params(VideoConfigParams {
        name: model.name.clone(),
        backend: model.backend.clone(),
        base_url: model.base_url.clone().unwrap_or_default(),
        api_key_encrypted: model.api_key_encrypted.clone(),
        model: model.model.clone(),
        max_concurrent: model.max_concurrent,
        default_width: model.default_width,
        default_height: model.default_height,
        default_duration_secs: model.default_duration_secs,
        min_duration_secs: model.min_duration_secs,
        max_duration_secs: model.max_duration_secs,
        duration_mode: model.duration_mode.clone(),
        supported_durations_json: model.supported_durations_json.clone(),
        default_fps: model.default_fps,
    })
    .map(|_| ())
    .map_err(a4n_error::AppError::InvalidArgument)
}

fn embedding_params(
    input: CreateEmbeddingInput,
) -> Result<(CreateEmbeddingConfigParams, Value), a4n_error::AppError> {
    let pricing = pricing_json(input.pricing)?;
    let params = validate_embedding_config_params(CreateEmbeddingConfigParams {
        name: input.name,
        backend: input.backend,
        base_url: input.base_url,
        api_key_encrypted: input.api_key_encrypted,
        model: input.model,
        dimension: input.dimension,
        batch_size: input.batch_size,
        local_model_path: input.local_model_path,
    })?;
    Ok((params, pricing))
}

fn embedding_record_input(
    params: CreateEmbeddingConfigParams,
    pricing_json: Value,
) -> EmbeddingConfigRecordInput {
    EmbeddingConfigRecordInput {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted: params.api_key_encrypted,
        model: params.model,
        dimension: params.dimension,
        batch_size: params.batch_size,
        local_model_path: params.local_model_path,
        pricing_json,
    }
}

fn merge_api_key_update(
    update: Option<Option<String>>,
    existing: Option<String>,
) -> Option<String> {
    update.unwrap_or(existing)
}
