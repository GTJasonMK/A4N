use a4n_core::{
    error::AppError,
    models::{
        ai::{embedding_config, image_gen_config, llm_config, system_settings, video_gen_config},
        user::theme_config,
    },
};
use sea_orm::{DatabaseConnection, EntityTrait};

use super::types::{
    SettingsAllConfigRows, SettingsAllEmbeddingConfigRow, SettingsAllImageConfigRow,
    SettingsAllLlmConfigRow, SettingsAllSystemSettingRow, SettingsAllThemeRow,
    SettingsAllVideoConfigRow,
};

pub async fn settings_all_config_rows(
    db: &DatabaseConnection,
) -> Result<SettingsAllConfigRows, AppError> {
    let llm_configs = llm_config::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(|row| SettingsAllLlmConfigRow {
            name: row.name,
            provider: row.provider,
            base_url: row.base_url,
            api_key_encrypted: row.api_key_encrypted,
            model: row.model,
            max_concurrent: row.max_concurrent,
            timeout_secs: row.timeout_secs,
            max_context_tokens: row.max_context_tokens,
            max_output_tokens: row.max_output_tokens,
            extra_body_json: row.extra_body_json,
            is_default: row.is_default,
        })
        .collect();
    let embedding_configs = embedding_config::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(|row| SettingsAllEmbeddingConfigRow {
            name: row.name,
            backend: row.backend,
            base_url: row.base_url,
            api_key_encrypted: row.api_key_encrypted,
            model: row.model,
            dimension: row.dimension,
            batch_size: row.batch_size,
            local_model_path: row.local_model_path,
            is_default: row.is_default,
        })
        .collect();
    let image_configs = image_gen_config::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(|row| {
            Ok(SettingsAllImageConfigRow {
                name: row.name,
                backend: row.backend,
                base_url: row.base_url.ok_or_else(|| {
                    AppError::ConfigError(format!("图片配置 {} 缺少 base_url", row.id))
                })?,
                api_key_encrypted: row.api_key_encrypted,
                model: row.model,
                max_concurrent: row.max_concurrent,
                default_width: row.default_width,
                default_height: row.default_height,
                is_default: row.is_default,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let video_configs = video_gen_config::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(|row| {
            Ok(SettingsAllVideoConfigRow {
                name: row.name,
                backend: row.backend,
                base_url: row.base_url.ok_or_else(|| {
                    AppError::ConfigError(format!("视频配置 {} 缺少 base_url", row.id))
                })?,
                api_key_encrypted: row.api_key_encrypted,
                model: row.model,
                max_concurrent: row.max_concurrent,
                default_width: row.default_width,
                default_height: row.default_height,
                default_duration_secs: row.default_duration_secs,
                min_duration_secs: row.min_duration_secs,
                max_duration_secs: row.max_duration_secs,
                duration_mode: row.duration_mode,
                supported_durations_json: row.supported_durations_json,
                default_fps: row.default_fps,
                is_default: row.is_default,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let themes = theme_config::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(|row| {
            Ok(SettingsAllThemeRow {
                user_id: row.user_id,
                name: row.name,
                mode: row.mode.parse().map_err(|error| {
                    AppError::ConfigError(format!("主题 {} 的持久化 mode 无效：{error}", row.id))
                })?,
                css_variables: serde_json::from_value(row.css_variables).map_err(|error| {
                    AppError::ConfigError(format!(
                        "主题 {} 的持久化 css_variables 无效：{error}",
                        row.id
                    ))
                })?,
                is_system_default: row.is_system_default,
                is_active: row.is_active,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let system_settings = system_settings::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(|row| SettingsAllSystemSettingRow {
            key: row.key,
            value: row.value,
            category: row.category,
            description: row.description,
        })
        .collect();

    Ok(SettingsAllConfigRows {
        llm_configs,
        embedding_configs,
        image_configs,
        video_configs,
        themes,
        system_settings,
    })
}
