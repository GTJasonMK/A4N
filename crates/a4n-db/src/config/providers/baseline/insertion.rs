use super::contracts::{BaselineImageConfigSeed, BaselineLlmConfigSeed, BaselineVideoConfigSeed};
use super::existence::{
    embedding_configs_exist, image_configs_exist, llm_configs_exist, video_configs_exist,
};
use a4n_core::{
    error::AppError,
    models::ai::{embedding_config, image_gen_config, llm_config, video_gen_config},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};

pub async fn insert_baseline_embedding_config_if_empty(
    db: &DatabaseConnection,
) -> Result<bool, AppError> {
    if embedding_configs_exist(db).await? {
        return Ok(false);
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
    Ok(true)
}

pub async fn insert_baseline_llm_config_if_empty(
    db: &DatabaseConnection,
    seed: BaselineLlmConfigSeed,
) -> Result<bool, AppError> {
    if llm_configs_exist(db).await? {
        return Ok(false);
    }

    let now = Utc::now().naive_utc();
    llm_config::ActiveModel {
        name: Set(seed.name),
        provider: Set(seed.provider),
        base_url: Set(seed.base_url),
        api_key_encrypted: Set(seed.api_key_encrypted),
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
    Ok(true)
}

pub async fn insert_baseline_image_config_if_empty(
    db: &DatabaseConnection,
    seed: BaselineImageConfigSeed,
) -> Result<bool, AppError> {
    if image_configs_exist(db).await? {
        return Ok(false);
    }

    let now = Utc::now().naive_utc();
    image_gen_config::ActiveModel {
        name: Set(seed.name),
        backend: Set(seed.backend),
        base_url: Set(Some(seed.base_url)),
        api_key_encrypted: Set(seed.api_key_encrypted),
        model: Set(seed.model),
        max_concurrent: Set(seed.max_concurrent),
        default_width: Set(seed.default_width),
        default_height: Set(seed.default_height),
        is_default: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;
    Ok(true)
}

pub async fn insert_baseline_video_config_if_empty(
    db: &DatabaseConnection,
    seed: BaselineVideoConfigSeed,
) -> Result<bool, AppError> {
    if video_configs_exist(db).await? {
        return Ok(false);
    }

    let now = Utc::now().naive_utc();
    video_gen_config::ActiveModel {
        name: Set(seed.name),
        backend: Set(seed.backend),
        base_url: Set(Some(seed.base_url)),
        api_key_encrypted: Set(seed.api_key_encrypted),
        model: Set(seed.model),
        max_concurrent: Set(seed.max_concurrent),
        default_width: Set(seed.default_width),
        default_height: Set(seed.default_height),
        default_duration_secs: Set(seed.default_duration_secs),
        min_duration_secs: Set(seed.min_duration_secs),
        max_duration_secs: Set(seed.max_duration_secs),
        duration_mode: Set(seed.duration_mode),
        supported_durations_json: Set(seed.supported_durations_json),
        default_fps: Set(seed.default_fps),
        is_default: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;
    Ok(true)
}
