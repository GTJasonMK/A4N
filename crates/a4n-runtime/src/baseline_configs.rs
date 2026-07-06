//! Empty-database baseline AI configs.
//!
//! Server-side public configs are seeded only when their table is empty. They
//! keep shared API keys on the backend while still allowing users to override
//! model settings from their local runtime settings snapshot.

use a4n_core::config::app_config::{AppConfig, ImageGenConfig, LlmConfig};
use a4n_core::error::AppError;
use a4n_core::models::ai::{embedding_config, image_gen_config, llm_config, video_gen_config};
use a4n_engine::llm::http::normalize_request_options_json;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, PaginatorTrait, Set};
use serde_json::Value;

pub async fn ensure_baseline_model_configs(
    db: &DatabaseConnection,
    config: &AppConfig,
) -> Result<(), AppError> {
    ensure_baseline_embedding_config(db).await?;
    ensure_public_llm_config(db, &config.llm).await?;
    ensure_public_image_config(db, &config.image_gen).await?;
    ensure_public_video_config(db).await?;
    Ok(())
}

async fn ensure_baseline_embedding_config(db: &DatabaseConnection) -> Result<(), AppError> {
    let count = embedding_config::Entity::find()
        .count(db)
        .await
        .map_err(AppError::Database)?;
    if count > 0 {
        return Ok(());
    }

    let now = Utc::now().naive_utc();
    embedding_config::ActiveModel {
        name: Set("基础本地哈希向量".to_string()),
        backend: Set("local_hash".to_string()),
        base_url: Set(None),
        api_key_encrypted: Set(None),
        model: Set("local-sha256-hash".to_string()),
        dimension: Set(384),
        batch_size: Set(100),
        local_model_path: Set(None),
        is_default: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

async fn ensure_public_llm_config(
    db: &DatabaseConnection,
    fallback: &LlmConfig,
) -> Result<(), AppError> {
    let count = llm_config::Entity::find()
        .count(db)
        .await
        .map_err(AppError::Database)?;
    if count > 0 {
        return Ok(());
    }
    let Some(seed) = public_llm_seed(fallback)? else {
        return Ok(());
    };

    let now = Utc::now().naive_utc();
    llm_config::ActiveModel {
        name: Set(seed.name),
        provider: Set(seed.provider),
        base_url: Set(seed.base_url),
        api_key_encrypted: Set(seed.api_key),
        model: Set(seed.model),
        max_concurrent: Set(seed.max_concurrent),
        timeout_secs: Set(seed.timeout_secs),
        max_context_tokens: Set(seed.max_context_tokens),
        max_output_tokens: Set(seed.max_output_tokens),
        request_options_json: Set(seed.request_options_json),
        is_default: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

async fn ensure_public_image_config(
    db: &DatabaseConnection,
    fallback: &ImageGenConfig,
) -> Result<(), AppError> {
    let count = image_gen_config::Entity::find()
        .count(db)
        .await
        .map_err(AppError::Database)?;
    if count > 0 {
        return Ok(());
    }
    let Some(seed) = public_image_seed(fallback)? else {
        return Ok(());
    };

    let params = a4n_services::config::validate_image_config_params(seed)?;
    let now = Utc::now().naive_utc();
    image_gen_config::ActiveModel {
        name: Set(params.name),
        backend: Set(params.backend),
        base_url: Set(Some(params.base_url)),
        api_key_encrypted: Set(params.api_key_encrypted),
        model: Set(params.model),
        max_concurrent: Set(params.max_concurrent),
        default_width: Set(params.default_width),
        default_height: Set(params.default_height),
        is_default: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

async fn ensure_public_video_config(db: &DatabaseConnection) -> Result<(), AppError> {
    let count = video_gen_config::Entity::find()
        .count(db)
        .await
        .map_err(AppError::Database)?;
    if count > 0 {
        return Ok(());
    }
    let Some(seed) = public_video_seed()? else {
        return Ok(());
    };

    let params = a4n_services::video::validate_video_config_params(seed)
        .map_err(AppError::InvalidArgument)?;
    let now = Utc::now().naive_utc();
    video_gen_config::ActiveModel {
        name: Set(params.name),
        backend: Set(params.backend),
        base_url: Set(Some(params.base_url)),
        api_key_encrypted: Set(params.api_key_encrypted),
        model: Set(params.model),
        max_concurrent: Set(params.max_concurrent),
        default_width: Set(params.default_width),
        default_height: Set(params.default_height),
        default_duration_secs: Set(params.default_duration_secs),
        min_duration_secs: Set(params.min_duration_secs),
        max_duration_secs: Set(params.max_duration_secs),
        duration_mode: Set(params.duration_mode),
        supported_durations_json: Set(params.supported_durations_json),
        default_fps: Set(params.default_fps),
        is_default: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

struct PublicLlmSeed {
    name: String,
    provider: String,
    base_url: String,
    api_key: Option<String>,
    model: String,
    max_concurrent: i32,
    timeout_secs: i32,
    max_context_tokens: i32,
    max_output_tokens: i32,
    request_options_json: Option<Value>,
}

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
    "A4N_PUBLIC_LLM_REQUEST_OPTIONS_JSON",
];

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

const PUBLIC_VIDEO_ENV_KEYS: &[&str] = &[
    "A4N_PUBLIC_VIDEO_NAME",
    "A4N_PUBLIC_VIDEO_BACKEND",
    "A4N_PUBLIC_VIDEO_BASE_URL",
    "A4N_PUBLIC_VIDEO_API_KEY",
    "A4N_PUBLIC_VIDEO_MODEL",
    "A4N_PUBLIC_VIDEO_MAX_CONCURRENT",
    "A4N_PUBLIC_VIDEO_DEFAULT_WIDTH",
    "A4N_PUBLIC_VIDEO_DEFAULT_HEIGHT",
    "A4N_PUBLIC_VIDEO_DEFAULT_DURATION_SECS",
    "A4N_PUBLIC_VIDEO_MIN_DURATION_SECS",
    "A4N_PUBLIC_VIDEO_MAX_DURATION_SECS",
    "A4N_PUBLIC_VIDEO_DURATION_MODE",
    "A4N_PUBLIC_VIDEO_SUPPORTED_DURATIONS_JSON",
    "A4N_PUBLIC_VIDEO_DEFAULT_FPS",
];

fn public_llm_seed(fallback: &LlmConfig) -> Result<Option<PublicLlmSeed>, AppError> {
    let public_api_key = optional_env_string("A4N_PUBLIC_LLM_API_KEY")?;
    let api_key = public_api_key.or_else(|| normalize_optional_string(fallback.api_key.clone()));
    let public_base_url = optional_env_string("A4N_PUBLIC_LLM_BASE_URL")?;
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
    let request_options_json = normalize_request_options_json(
        optional_env_json("A4N_PUBLIC_LLM_REQUEST_OPTIONS_JSON")?
            .or_else(|| fallback.request_options_json.clone()),
    )
    .map_err(|err| {
        AppError::ConfigError(format!(
            "A4N_PUBLIC_LLM_REQUEST_OPTIONS_JSON 无效：{}",
            app_error_plain_message(err)
        ))
    })?;

    Ok(Some(PublicLlmSeed {
        name: required_seed_value(name, "A4N_PUBLIC_LLM_NAME")?,
        provider: required_seed_value(provider, "A4N_PUBLIC_LLM_PROVIDER")?,
        base_url: required_seed_value(base_url, "A4N_PUBLIC_LLM_BASE_URL")?,
        api_key,
        model: required_seed_value(model, "A4N_PUBLIC_LLM_MODEL")?,
        max_concurrent: positive_i32(max_concurrent, "A4N_PUBLIC_LLM_MAX_CONCURRENT")?,
        timeout_secs: positive_i32(timeout_secs, "A4N_PUBLIC_LLM_TIMEOUT_SECS")?,
        max_context_tokens: positive_i32(max_context_tokens, "A4N_PUBLIC_LLM_MAX_CONTEXT_TOKENS")?,
        max_output_tokens: positive_i32(max_output_tokens, "A4N_PUBLIC_LLM_MAX_OUTPUT_TOKENS")?,
        request_options_json,
    }))
}

fn public_image_seed(
    fallback: &ImageGenConfig,
) -> Result<Option<a4n_services::config::CreateImageConfigParams>, AppError> {
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

    Ok(Some(a4n_services::config::CreateImageConfigParams {
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

fn public_video_seed() -> Result<Option<a4n_services::video::VideoConfigParams>, AppError> {
    if !any_env_present(PUBLIC_VIDEO_ENV_KEYS)? {
        return Ok(None);
    }
    let base_url = required_env_string("A4N_PUBLIC_VIDEO_BASE_URL")?;
    let api_key = required_env_string("A4N_PUBLIC_VIDEO_API_KEY")?;
    let model = required_env_string("A4N_PUBLIC_VIDEO_MODEL")?;

    Ok(Some(a4n_services::video::VideoConfigParams {
        name: env_string_or("A4N_PUBLIC_VIDEO_NAME", "公共基础视频")?,
        backend: env_string_or("A4N_PUBLIC_VIDEO_BACKEND", "openai-compatible-video")?,
        base_url,
        api_key_encrypted: Some(api_key),
        model,
        max_concurrent: env_i32_or("A4N_PUBLIC_VIDEO_MAX_CONCURRENT", 1)?,
        default_width: env_i32_or("A4N_PUBLIC_VIDEO_DEFAULT_WIDTH", 1280)?,
        default_height: env_i32_or("A4N_PUBLIC_VIDEO_DEFAULT_HEIGHT", 720)?,
        default_duration_secs: env_i32_or("A4N_PUBLIC_VIDEO_DEFAULT_DURATION_SECS", 5)?,
        min_duration_secs: env_i32_or("A4N_PUBLIC_VIDEO_MIN_DURATION_SECS", 1)?,
        max_duration_secs: env_i32_or("A4N_PUBLIC_VIDEO_MAX_DURATION_SECS", 60)?,
        duration_mode: env_string_or("A4N_PUBLIC_VIDEO_DURATION_MODE", "range")?,
        supported_durations_json: optional_env_json("A4N_PUBLIC_VIDEO_SUPPORTED_DURATIONS_JSON")?,
        default_fps: env_i32_or("A4N_PUBLIC_VIDEO_DEFAULT_FPS", 24)?,
        provider_options_json: None,
    }))
}

fn any_env_present(keys: &[&str]) -> Result<bool, AppError> {
    for key in keys {
        if optional_env_string(key)?.is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn required_env_string(key: &str) -> Result<String, AppError> {
    optional_env_string(key)?
        .ok_or_else(|| AppError::ConfigError(format!("公共默认配置缺少 {key}")))
}

fn env_string_or(key: &str, fallback: &str) -> Result<String, AppError> {
    Ok(optional_env_string(key)?.unwrap_or_else(|| fallback.trim().to_string()))
}

fn optional_env_string(key: &str) -> Result<Option<String>, AppError> {
    match std::env::var(key) {
        Ok(value) => {
            let value = value.trim().to_string();
            if value.is_empty() {
                Err(AppError::ConfigError(format!("{key} 不能为空")))
            } else {
                Ok(Some(value))
            }
        }
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(err) => Err(AppError::ConfigError(format!("{key} 无法读取: {err}"))),
    }
}

fn optional_env_json(key: &str) -> Result<Option<Value>, AppError> {
    let Some(value) = optional_env_string(key)? else {
        return Ok(None);
    };
    serde_json::from_str(&value)
        .map(Some)
        .map_err(|err| AppError::ConfigError(format!("{key} 必须是合法 JSON：{err}")))
}

fn app_error_plain_message(err: AppError) -> String {
    match err {
        AppError::InvalidArgument(message)
        | AppError::ConfigError(message)
        | AppError::LlmError(message)
        | AppError::Timeout(message) => message,
        other => other.to_string(),
    }
}

fn env_i32_or(key: &str, fallback: i32) -> Result<i32, AppError> {
    let Some(value) = optional_env_string(key)? else {
        return Ok(fallback);
    };
    value
        .parse::<i32>()
        .map_err(|err| AppError::ConfigError(format!("{key} 必须是整数：{err}")))
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn required_seed_value(value: String, key: &str) -> Result<String, AppError> {
    if value.trim().is_empty() {
        Err(AppError::ConfigError(format!("{key} 不能为空")))
    } else {
        Ok(value)
    }
}

fn positive_i32(value: i32, key: &str) -> Result<i32, AppError> {
    if value <= 0 {
        Err(AppError::ConfigError(format!("{key} 必须大于 0")))
    } else {
        Ok(value)
    }
}

fn usize_to_i32(value: usize, key: &str) -> Result<i32, AppError> {
    i32::try_from(value).map_err(|_| AppError::ConfigError(format!("{key} 超出 i32 范围")))
}

fn u64_to_i32(value: u64, key: &str) -> Result<i32, AppError> {
    i32::try_from(value).map_err(|_| AppError::ConfigError(format!("{key} 超出 i32 范围")))
}

fn u32_to_i32(value: u32, key: &str) -> Result<i32, AppError> {
    i32::try_from(value).map_err(|_| AppError::ConfigError(format!("{key} 超出 i32 范围")))
}
