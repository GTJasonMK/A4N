fn pricing_json(
    pricing: a4n_api_contract::config::PricingPlan,
) -> Result<Value, a4n_error::AppError> {
    let pricing = pricing
        .validate()
        .map_err(a4n_error::AppError::InvalidArgument)?;
    serde_json::to_value(pricing).map_err(a4n_error::AppError::Serialization)
}

fn normalize_llm_input(input: CreateLlmInput) -> Result<NormalizedLlmConfig, a4n_error::AppError> {
    let CreateLlmInput {
        name,
        provider,
        base_url,
        api_key_encrypted,
        model,
        max_concurrent,
        timeout_secs,
        max_context_tokens,
        max_output_tokens,
        agent_protocol,
        extra_body_json,
        pricing,
    } = input;
    let mut normalized = normalize_create_llm_config_input(CreateLlmConfigInput {
        name: &name,
        provider: &provider,
        base_url: &base_url,
        api_key_encrypted: api_key_encrypted.as_deref(),
        model: &model,
        max_concurrent,
        timeout_secs,
        max_context_tokens,
        max_output_tokens,
        agent_protocol,
        extra_body_json,
    })?;
    normalized.pricing_json = pricing_json(pricing)?;
    Ok(normalized)
}

fn normalize_update_llm_input(
    existing: &a4n_db_models::models::ai::llm_config::Model,
    input: UpdateLlmInput,
) -> Result<NormalizedLlmConfig, a4n_error::AppError> {
    let mut normalized = normalize_update_llm_config_input(
        existing.api_key_encrypted.as_deref(),
        UpdateLlmConfigInput {
            name: &input.name,
            provider: &input.provider,
            base_url: &input.base_url,
            api_key_encrypted: input
                .api_key_encrypted
                .as_ref()
                .map(|value| value.as_deref()),
            model: &input.model,
            max_concurrent: input.max_concurrent,
            timeout_secs: input.timeout_secs,
            max_context_tokens: input.max_context_tokens,
            max_output_tokens: input.max_output_tokens,
            agent_protocol: input.agent_protocol,
            extra_body_json: input.extra_body_json,
        },
    )?;
    normalized.pricing_json = pricing_json(input.pricing)?;
    Ok(normalized)
}

fn llm_record_input(normalized: NormalizedLlmConfig) -> LlmConfigRecordInput {
    LlmConfigRecordInput {
        name: normalized.name,
        provider: normalized.provider,
        base_url: normalized.base_url,
        api_key_encrypted: normalized.api_key_encrypted,
        model: normalized.model,
        max_concurrent: normalized.max_concurrent,
        timeout_secs: normalized.timeout_secs,
        max_context_tokens: normalized.max_context_tokens,
        max_output_tokens: normalized.max_output_tokens,
        agent_protocol: normalized.agent_protocol,
        extra_body_json: normalized.extra_body_json,
        pricing_json: normalized.pricing_json,
    }
}

fn llm_config_snapshot(model: &a4n_db_models::models::ai::llm_config::Model) -> LlmConfigSnapshot {
    LlmConfigSnapshot {
        name: model.name.clone(),
        provider: model.provider.clone(),
        base_url: model.base_url.clone(),
        api_key_encrypted: model.api_key_encrypted.clone(),
        model: model.model.clone(),
        max_concurrent: model.max_concurrent,
        timeout_secs: model.timeout_secs,
        max_context_tokens: model.max_context_tokens,
        max_output_tokens: model.max_output_tokens,
        agent_protocol: model.agent_protocol.clone(),
        extra_body_json: model.extra_body_json.clone(),
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
    let params = a4n_config::validate_embedding_config_params(CreateEmbeddingConfigParams {
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

fn parse_settings_export(settings_json: &str) -> Result<AllConfigsExport, WorkerFailure> {
    serde_json::from_str(settings_json)
        .map_err(|error| WorkerFailure::invalid_request(format!("JSON 解析失败：{error}")))
}

fn settings_import_input(
    incoming: &AllConfigsExport,
    overwrite: bool,
) -> SettingsAllConfigImportInput {
    SettingsAllConfigImportInput {
        overwrite,
        rows: SettingsAllConfigRows {
            llm_configs: incoming.llm_configs.clone(),
            embedding_configs: incoming.embedding_configs.clone(),
            image_configs: incoming.image_configs.clone(),
            video_configs: incoming.video_configs.clone(),
            themes: incoming.themes.clone(),
            system_settings: incoming.system_settings.clone(),
        },
    }
}

fn settings_import_error(error: a4n_db::config::SettingsAllConfigImportError) -> WorkerFailure {
    match error {
        a4n_db::config::SettingsAllConfigImportError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::config::SettingsAllConfigImportError::Database { source, .. } => {
            WorkerFailure::unavailable(source.to_string())
        }
    }
}

async fn validate_admin_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
    database: &DbHandle,
) -> Result<(), WorkerFailure> {
    validate_user_request(request, operation)?;
    require_admin(database, user_id(request)?).await
}

fn validate_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    request
        .validate()
        .map_err(|error| WorkerFailure::invalid_request(error.to_string()))?;
    if request.operation_id != operation {
        return Err(WorkerFailure::invalid_request(format!(
            "operation_id 不匹配: expected={operation}, actual={}",
            request.operation_id
        )));
    }
    if request.lease.worker_id != CONFIG_READ_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 config read worker",
        ));
    }
    Ok(())
}

fn validate_service_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    validate_request(request, operation)?;
    if request.principal.kind != WorkerPrincipalKind::Service {
        return Err(WorkerFailure::unauthorized("仅允许 service principal"));
    }
    Ok(())
}

fn validate_user_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    validate_request(request, operation)?;
    if request.principal.kind != WorkerPrincipalKind::User {
        return Err(WorkerFailure::unauthorized("仅允许用户 principal"));
    }
    Ok(())
}

fn user_id<T: serde::Serialize>(request: &WorkerEnvelope<T>) -> Result<i64, WorkerFailure> {
    request
        .principal
        .subject
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| WorkerFailure::unauthorized("用户 principal subject 无效"))
}

fn positive_id(value: i64) -> Result<i64, WorkerFailure> {
    if value > 0 {
        Ok(value)
    } else {
        Err(WorkerFailure::invalid_request("配置 ID 必须为正整数"))
    }
}

async fn require_admin(database: &DbHandle, user_id: i64) -> Result<(), WorkerFailure> {
    let user = a4n_db::auth::user_by_id(database.connection(), user_id)
        .await
        .map_err(app_error)?;
    if !user.is_active {
        return Err(WorkerFailure::unauthorized("用户已停用"));
    }
    if parse_user_role(&user.role)
        .map_err(|error| app_error(a4n_error::AppError::InvalidArgument(error)))?
        != UserRole::Admin
    {
        return Err(WorkerFailure::unauthorized("需要管理员配置权限"));
    }
    Ok(())
}

fn app_error(error: a4n_error::AppError) -> WorkerFailure {
    match error {
        a4n_error::AppError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        a4n_error::AppError::NotFound(message) => WorkerFailure::not_found(message),
        a4n_error::AppError::Forbidden(message) | a4n_error::AppError::Unauthorized(message) => {
            WorkerFailure::unauthorized(message)
        }
        a4n_error::AppError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{current}: {operation}"))
        }
        other => WorkerFailure::unavailable(other.to_string()),
    }
}

impl WorkerFailure {
    fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::InvalidRequest,
            message: message.into(),
            retryable: false,
        }
    }
    fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Unauthorized,
            message: message.into(),
            retryable: false,
        }
    }
    fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::NotFound,
            message: message.into(),
            retryable: false,
        }
    }
    fn conflict(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Conflict,
            message: message.into(),
            retryable: false,
        }
    }
    fn unavailable(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Unavailable,
            message: message.into(),
            retryable: true,
        }
    }
}

fn completed<T>(request: &WorkerEnvelope<impl serde::Serialize>, payload: T) -> WorkerResponse<T> {
    WorkerResponse {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: request.request_id.clone(),
        trace_id: request.trace_id.clone(),
        operation: WorkerOperationRef {
            operation_id: request.operation_id.clone(),
            task_id: None,
        },
        status: WorkerOperationStatus::Completed,
        payload: Some(payload),
        resource_refs: Vec::new(),
        committed_revision: None,
        provider_job_id: None,
        result_summary: None,
        pagination: None,
        error: None,
    }
}

fn failed<T>(
    request: &WorkerEnvelope<impl serde::Serialize>,
    failure: WorkerFailure,
) -> WorkerResponse<T> {
    WorkerResponse {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: request.request_id.clone(),
        trace_id: request.trace_id.clone(),
        operation: WorkerOperationRef {
            operation_id: request.operation_id.clone(),
            task_id: None,
        },
        status: WorkerOperationStatus::Failed,
        payload: None,
        resource_refs: Vec::new(),
        committed_revision: None,
        provider_job_id: None,
        result_summary: None,
        pagination: None,
        error: Some(WorkerError {
            code: failure.code,
            message: failure.message,
            retryable: failure.retryable,
        }),
    }
}

fn unauthorized() -> Response {
    StatusCode::UNAUTHORIZED.into_response()
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .try_init();
}
