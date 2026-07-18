use super::env::{
    any_env_present, env_i32_or, env_string_or, normalize_optional_string, optional_env_string,
    u32_to_i32, usize_to_i32,
};
use a4n_core::{config::app_config::ImageGenConfig, error::AppError};
use a4n_orchestrator::{OrchestratorContext, config::BaselineImageConfigSeed};

const PUBLIC_IMAGE_ENV_KEYS: &[&str] = &[
    "A4N_PUBLIC_IMAGE_NAME",
    "A4N_PUBLIC_IMAGE_BACKEND",
    "A4N_PUBLIC_IMAGE_BASE_URL",
    "A4N_PUBLIC_IMAGE_API_KEY",
    "A4N_PUBLIC_IMAGE_MODEL",
    "A4N_PUBLIC_IMAGE_MAX_CONCURRENT",
    "A4N_PUBLIC_IMAGE_DEFAULT_WIDTH",
    "A4N_PUBLIC_IMAGE_DEFAULT_HEIGHT",
];

pub(super) async fn ensure_public_image_config(
    orchestrator: &OrchestratorContext,
    fallback: &ImageGenConfig,
) -> Result<(), AppError> {
    if orchestrator.image_configs_exist().await? {
        return Ok(());
    }
    let Some(seed) = public_image_seed(fallback)? else {
        return Ok(());
    };
    let params = a4n_core::config::validate_image_config_params(seed)?;
    orchestrator
        .insert_baseline_image_config_if_empty(BaselineImageConfigSeed {
            name: params.name,
            backend: params.backend,
            base_url: params.base_url,
            api_key_encrypted: params.api_key_encrypted,
            model: params.model,
            max_concurrent: params.max_concurrent,
            default_width: params.default_width,
            default_height: params.default_height,
        })
        .await?;
    Ok(())
}

fn public_image_seed(
    fallback: &ImageGenConfig,
) -> Result<Option<a4n_core::config::CreateImageConfigParams>, AppError> {
    let api_key = match optional_env_string("A4N_PUBLIC_IMAGE_API_KEY")?
        .or_else(|| normalize_optional_string(fallback.api_key.clone()))
    {
        Some(api_key) => Some(api_key),
        None => optional_env_string("A4N_OPENAI_API_KEY")?,
    };
    let public_base_url = optional_env_string("A4N_PUBLIC_IMAGE_BASE_URL")?;
    let requested =
        api_key.is_some() || public_base_url.is_some() || any_env_present(PUBLIC_IMAGE_ENV_KEYS)?;
    if !requested {
        return Ok(None);
    }
    let backend = env_string_or("A4N_PUBLIC_IMAGE_BACKEND", &fallback.backend)?;
    let shared_openai_base_url = if public_base_url.is_none() && fallback.base_url.is_none() {
        optional_env_string("A4N_OPENAI_BASE_URL")?
    } else {
        None
    };
    let base_url = public_base_url
        .or_else(|| normalize_optional_string(fallback.base_url.clone()))
        .or(shared_openai_base_url)
        .or_else(|| {
            if api_key.is_some() && backend.starts_with("openai-") {
                Some("https://api.openai.com/v1".to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| {
            AppError::ConfigError("公共图片配置缺少 A4N_PUBLIC_IMAGE_BASE_URL".into())
        })?;
    let default_width = env_i32_or(
        "A4N_PUBLIC_IMAGE_DEFAULT_WIDTH",
        u32_to_i32(fallback.default_width, "A4N_PUBLIC_IMAGE_DEFAULT_WIDTH")?,
    )?;
    let default_height = env_i32_or(
        "A4N_PUBLIC_IMAGE_DEFAULT_HEIGHT",
        u32_to_i32(fallback.default_height, "A4N_PUBLIC_IMAGE_DEFAULT_HEIGHT")?,
    )?;
    Ok(Some(a4n_core::config::CreateImageConfigParams {
        name: env_string_or("A4N_PUBLIC_IMAGE_NAME", "公共基础图片")?,
        backend,
        base_url,
        api_key_encrypted: api_key,
        model: env_string_or("A4N_PUBLIC_IMAGE_MODEL", &fallback.model)?,
        max_concurrent: env_i32_or(
            "A4N_PUBLIC_IMAGE_MAX_CONCURRENT",
            usize_to_i32(fallback.max_concurrent, "A4N_PUBLIC_IMAGE_MAX_CONCURRENT")?,
        )?,
        default_width,
        default_height,
        provider_options_json: None,
    }))
}
