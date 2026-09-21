use super::env::{any_env_present, env_i32_or, env_string_or, usize_to_i32};
use a4n_api_contract::config::{
    ConfigMutationApplicationPort, ConfigReadApplicationPort, ImageConfigInput,
};
use a4n_config::{
    app_config::ImageGenConfig, normalize_optional_config_value, optional_non_empty_env_string,
};
use a4n_error::AppError;

const PUBLIC_IMAGE_ENV_KEYS: &[&str] = &[
    "A4N_PUBLIC_IMAGE_NAME",
    "A4N_PUBLIC_IMAGE_BACKEND",
    "A4N_PUBLIC_IMAGE_BASE_URL",
    "A4N_PUBLIC_IMAGE_API_KEY",
    "A4N_PUBLIC_IMAGE_MODEL",
    "A4N_PUBLIC_IMAGE_MAX_CONCURRENT",
];

pub(super) async fn ensure_public_image_config(
    config_read_application: &dyn ConfigReadApplicationPort,
    config_mutation_application: &dyn ConfigMutationApplicationPort,
    fallback: &ImageGenConfig,
) -> Result<(), AppError> {
    if config_read_application.image_configs_exist().await? {
        return Ok(());
    }
    let Some(seed) = public_image_seed(fallback)? else {
        return Ok(());
    };
    config_mutation_application
        .insert_baseline_image_if_empty(seed)
        .await?;
    Ok(())
}

fn public_image_seed(fallback: &ImageGenConfig) -> Result<Option<ImageConfigInput>, AppError> {
    let api_key = match optional_non_empty_env_string("A4N_PUBLIC_IMAGE_API_KEY")?
        .or_else(|| normalize_optional_config_value(fallback.api_key.clone()))
    {
        Some(api_key) => Some(api_key),
        None => optional_non_empty_env_string("A4N_OPENAI_API_KEY")?,
    };
    let public_base_url = optional_non_empty_env_string("A4N_PUBLIC_IMAGE_BASE_URL")?;
    let requested =
        api_key.is_some() || public_base_url.is_some() || any_env_present(PUBLIC_IMAGE_ENV_KEYS)?;
    if !requested {
        return Ok(None);
    }
    let backend = env_string_or("A4N_PUBLIC_IMAGE_BACKEND", &fallback.backend)?;
    let shared_openai_base_url = if public_base_url.is_none() && fallback.base_url.is_none() {
        optional_non_empty_env_string("A4N_OPENAI_BASE_URL")?
    } else {
        None
    };
    let base_url = public_base_url
        .or_else(|| normalize_optional_config_value(fallback.base_url.clone()))
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
    Ok(Some(ImageConfigInput {
        name: env_string_or("A4N_PUBLIC_IMAGE_NAME", "公共基础图片")?,
        backend,
        base_url,
        api_key_encrypted: api_key,
        model: env_string_or("A4N_PUBLIC_IMAGE_MODEL", &fallback.model)?,
        max_concurrent: env_i32_or(
            "A4N_PUBLIC_IMAGE_MAX_CONCURRENT",
            usize_to_i32(fallback.max_concurrent, "A4N_PUBLIC_IMAGE_MAX_CONCURRENT")?,
        )?,
        supported_sizes: fallback.supported_sizes.clone(),
        pricing: Default::default(),
    }))
}
