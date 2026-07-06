use super::types::{
    AllConfigsExport, EmbeddingRow, ImageRow, LlmRow, PromptRow, SystemSettingRow, ThemeRow,
    VideoRow,
};
use crate::AppState;
use a4n_core::error::AppError;
use a4n_core::models::ai::{
    embedding_config, image_gen_config, llm_config, system_settings, video_gen_config,
};
use a4n_core::models::user::theme_config;
use chrono::Utc;
use sea_orm::EntityTrait;

pub async fn export_all_configs_to_json(state: &AppState) -> Result<String, AppError> {
    export_all_configs_json(state).await
}

async fn export_all_configs_json(state: &AppState) -> Result<String, AppError> {
    let db = state.db.as_ref();

    let llm_rows: Vec<LlmRow> = llm_config::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(|m| LlmRow {
            name: m.name,
            provider: m.provider,
            base_url: m.base_url,
            api_key_encrypted: m.api_key_encrypted,
            model: m.model,
            max_concurrent: m.max_concurrent,
            timeout_secs: m.timeout_secs,
            max_context_tokens: m.max_context_tokens,
            max_output_tokens: m.max_output_tokens,
            request_options_json: m.request_options_json,
            is_default: m.is_default,
        })
        .collect();

    let embedding_rows: Vec<EmbeddingRow> = embedding_config::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(|m| EmbeddingRow {
            name: m.name,
            backend: m.backend,
            base_url: m.base_url,
            api_key_encrypted: m.api_key_encrypted,
            model: m.model,
            dimension: m.dimension,
            batch_size: m.batch_size,
            local_model_path: m.local_model_path,
            is_default: m.is_default,
        })
        .collect();

    let image_rows: Vec<ImageRow> = image_gen_config::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(|m| ImageRow {
            name: m.name,
            backend: m.backend,
            base_url: m.base_url,
            api_key_encrypted: m.api_key_encrypted,
            model: m.model,
            max_concurrent: m.max_concurrent,
            default_width: m.default_width,
            default_height: m.default_height,
            provider_options_json: m.provider_options_json,
            is_default: m.is_default,
        })
        .collect();

    let video_rows: Vec<VideoRow> = video_gen_config::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(|m| VideoRow {
            name: m.name,
            backend: m.backend,
            base_url: m.base_url,
            api_key_encrypted: m.api_key_encrypted,
            model: m.model,
            max_concurrent: m.max_concurrent,
            default_width: m.default_width,
            default_height: m.default_height,
            default_duration_secs: m.default_duration_secs,
            min_duration_secs: m.min_duration_secs,
            max_duration_secs: m.max_duration_secs,
            duration_mode: m.duration_mode,
            supported_durations_json: m.supported_durations_json,
            default_fps: m.default_fps,
            provider_options_json: m.provider_options_json,
            is_default: m.is_default,
        })
        .collect();

    let prompts_rows: Vec<PromptRow> = state
        .engine
        .list_all()
        .into_iter()
        .filter(|p| p.is_modified)
        .map(|p| PromptRow {
            name: p.name,
            content: p.content,
            title: Some(p.title),
            description: p.description,
        })
        .collect();

    let theme_rows: Vec<ThemeRow> = theme_config::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(|m| ThemeRow {
            user_id: m.user_id,
            name: m.name,
            mode: m.mode,
            css_variables: m.css_variables,
            is_system_default: m.is_system_default,
            is_active: m.is_active,
        })
        .collect();

    let setting_rows: Vec<SystemSettingRow> = system_settings::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(|m| SystemSettingRow {
            key: m.key,
            value: m.value,
            category: m.category,
            description: m.description,
        })
        .collect();

    let export = AllConfigsExport {
        version: "1.0".into(),
        export_time: Utc::now().to_rfc3339(),
        export_type: "all".into(),
        llm_configs: llm_rows,
        embedding_configs: embedding_rows,
        image_configs: image_rows,
        video_configs: video_rows,
        prompts_modified: prompts_rows,
        themes: theme_rows,
        system_settings: setting_rows,
    };

    serde_json::to_string_pretty(&export).map_err(AppError::Serialization)
}
