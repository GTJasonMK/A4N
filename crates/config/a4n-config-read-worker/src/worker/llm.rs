use a4n_api_contract::config::{CreateLlmInput, UpdateLlmInput};
use a4n_config_read_worker_contract::{
    CONFIG_MUTATION_BASELINE_LLM_OPERATION, CONFIG_MUTATION_CREATE_LLM_OPERATION,
    CONFIG_MUTATION_DELETE_LLM_OPERATION, CONFIG_MUTATION_SET_DEFAULT_LLM_OPERATION,
    CONFIG_MUTATION_UPDATE_LLM_OPERATION,
};
use a4n_db::config::LlmConfigRecordInput;
use a4n_domain::config::LlmConfigReadRecord;
use a4n_db::config::read_records::{
    create_llm_config_read_record, llm_config_read_record_by_id, update_llm_config_read_record,
};
use a4n_llm_config::{
    CreateLlmConfigInput, LlmConfigSnapshot, NormalizedLlmConfig, UpdateLlmConfigInput,
    normalize_create_llm_config_input, normalize_update_llm_config_input, validate_llm_config,
};

impl WorkerState {
    async fn baseline_llm(
        &self,
        request: WorkerEnvelope<CreateLlmCommand>,
    ) -> Result<bool, WorkerFailure> {
        validate_service_request(&request, CONFIG_MUTATION_BASELINE_LLM_OPERATION)?;
        let input = llm_record_input(normalize_llm_input(request.payload.0).map_err(app_error)?);
        a4n_db::config::insert_baseline_llm_config_if_empty(self.database.executor(), input)
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
        let model = create_llm_config_read_record(
            self.database.executor(),
            llm_record_input(normalize_llm_input(request.payload.0).map_err(app_error)?),
        )
        .await
        .map_err(app_error)?;
        llm_summary(&model).map_err(app_error)
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
        let existing = llm_config_read_record_by_id(
            self.database.executor(),
            positive_id(request.payload.config_id)?,
        )
        .await
        .map_err(app_error)?;
        let input =
            normalize_update_llm_input(&existing, request.payload.input).map_err(app_error)?;
        let model = update_llm_config_read_record(
            self.database.executor(),
            &existing,
            llm_record_input(input),
        )
        .await
        .map_err(app_error)?;
        llm_summary(&model).map_err(app_error)
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
        let model = llm_config_read_record_by_id(self.database.executor(), config_id)
            .await
            .map_err(app_error)?;
        validate_llm_config(&llm_config_snapshot(&model)).map_err(app_error)?;
        a4n_db::config::set_default_provider_config(
            self.database.executor(),
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
        let model = llm_config_read_record_by_id(self.database.executor(), config_id)
            .await
            .map_err(app_error)?;
        ensure_default_config_delete_allowed(" LLM 配置", model.is_default).map_err(app_error)?;
        a4n_db::config::delete_llm_config_record(self.database.executor(), config_id)
            .await
            .map_err(app_error)
    }
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
    existing: &LlmConfigReadRecord,
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

fn llm_config_snapshot(model: &LlmConfigReadRecord) -> LlmConfigSnapshot {
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
