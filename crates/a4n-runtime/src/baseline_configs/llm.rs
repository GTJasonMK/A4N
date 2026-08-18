use super::env::{
    any_env_present, env_i32_or, env_string_or, optional_env_json, u64_to_i32, usize_to_i32,
};
use a4n_core::{
    config::{
        app_config::LlmConfig, app_error_plain_message, env::optional_non_empty_env_string,
        normalize_optional_config_value,
    },
    error::AppError,
};
use a4n_llm::config::{
    CreateLlmConfigInput, NormalizedLlmConfig, normalize_create_llm_config_input,
};
use a4n_orchestrator::OrchestratorContext;

const PUBLIC_LLM_ENV_KEYS: &[&str] = &[
    "A4N_PUBLIC_LLM_NAME",
    "A4N_PUBLIC_LLM_PROVIDER",
    "A4N_PUBLIC_LLM_BASE_URL",
    "A4N_PUBLIC_LLM_API_KEY",
    "A4N_PUBLIC_LLM_MODEL",
    "A4N_PUBLIC_LLM_MAX_CONCURRENT",
    "A4N_PUBLIC_LLM_TIMEOUT_SECS",
    "A4N_PUBLIC_LLM_MAX_CONTEXT_TOKENS",
    "A4N_PUBLIC_LLM_MAX_OUTPUT_TOKENS",
    "A4N_PUBLIC_LLM_EXTRA_BODY_JSON",
];

pub(super) async fn ensure_public_llm_config(
    orchestrator: &OrchestratorContext,
    fallback: &LlmConfig,
) -> Result<(), AppError> {
    if orchestrator.llm_configs_exist().await? {
        return Ok(());
    }
    let Some(seed) = public_llm_seed(fallback)? else {
        return Ok(());
    };
    orchestrator
        .insert_baseline_llm_config_if_empty(seed)
        .await?;
    Ok(())
}

fn public_llm_seed(fallback: &LlmConfig) -> Result<Option<NormalizedLlmConfig>, AppError> {
    let public_api_key = optional_non_empty_env_string("A4N_PUBLIC_LLM_API_KEY")?;
    let api_key =
        public_api_key.or_else(|| normalize_optional_config_value(fallback.api_key.clone()));
    let public_base_url = optional_non_empty_env_string("A4N_PUBLIC_LLM_BASE_URL")?;
    let requested =
        api_key.is_some() || public_base_url.is_some() || any_env_present(PUBLIC_LLM_ENV_KEYS)?;
    if !requested {
        return Ok(None);
    }
    if api_key.is_none() && public_base_url.is_none() {
        return Err(AppError::ConfigError(
            "已配置公共 LLM 环境变量，但缺少 A4N_PUBLIC_LLM_API_KEY 或 A4N_PUBLIC_LLM_BASE_URL"
                .into(),
        ));
    }
    let name = env_string_or("A4N_PUBLIC_LLM_NAME", "公共基础 LLM")?;
    let provider = env_string_or("A4N_PUBLIC_LLM_PROVIDER", &fallback.primary_provider)?;
    let base_url = public_base_url.unwrap_or_else(|| fallback.base_url.trim().to_string());
    let model = env_string_or("A4N_PUBLIC_LLM_MODEL", &fallback.model)?;
    let max_concurrent = env_i32_or(
        "A4N_PUBLIC_LLM_MAX_CONCURRENT",
        usize_to_i32(fallback.max_concurrent, "A4N_PUBLIC_LLM_MAX_CONCURRENT")?,
    )?;
    let timeout_secs = env_i32_or(
        "A4N_PUBLIC_LLM_TIMEOUT_SECS",
        u64_to_i32(fallback.timeout_secs, "A4N_PUBLIC_LLM_TIMEOUT_SECS")?,
    )?;
    let max_context_tokens = env_i32_or(
        "A4N_PUBLIC_LLM_MAX_CONTEXT_TOKENS",
        usize_to_i32(
            fallback.max_context_tokens,
            "A4N_PUBLIC_LLM_MAX_CONTEXT_TOKENS",
        )?,
    )?;
    let max_output_tokens = env_i32_or(
        "A4N_PUBLIC_LLM_MAX_OUTPUT_TOKENS",
        usize_to_i32(
            fallback.max_output_tokens,
            "A4N_PUBLIC_LLM_MAX_OUTPUT_TOKENS",
        )?,
    )?;
    normalize_create_llm_config_input(CreateLlmConfigInput {
        name: &name,
        provider: &provider,
        base_url: &base_url,
        api_key_encrypted: api_key.as_deref(),
        model: &model,
        max_concurrent,
        timeout_secs,
        max_context_tokens,
        max_output_tokens,
        extra_body_json: optional_env_json("A4N_PUBLIC_LLM_EXTRA_BODY_JSON")?
            .or_else(|| fallback.extra_body_json.clone()),
    })
    .map(Some)
    .map_err(|err| {
        AppError::ConfigError(format!(
            "公共 LLM 配置无效：{}",
            app_error_plain_message(err)
        ))
    })
}
