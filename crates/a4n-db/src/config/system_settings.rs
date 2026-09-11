use a4n_db_models::models::ai::system_settings;
use a4n_domain::LlmMaxTokensConfig;
use a4n_error::AppError;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

pub const LLM_MAX_TOKENS_SETTING_KEY: &str = "llm_max_tokens";

pub async fn llm_max_tokens_config<C>(db: &C) -> Result<LlmMaxTokensConfig, AppError>
where
    C: ConnectionTrait,
{
    let Some(setting) = system_settings::Entity::find()
        .filter(system_settings::Column::Key.eq(LLM_MAX_TOKENS_SETTING_KEY))
        .one(db)
        .await
        .map_err(AppError::Database)?
    else {
        return Ok(LlmMaxTokensConfig::default());
    };

    let config: LlmMaxTokensConfig = serde_json::from_value(setting.value)
        .map_err(|err| AppError::ConfigError(format!("LLM max_tokens 配置解析失败: {err}")))?;
    config
        .validate()
        .map_err(|err| AppError::ConfigError(format!("LLM max_tokens 配置无效: {err}")))?;
    Ok(config)
}

pub async fn save_llm_max_tokens_config<C>(
    db: &C,
    config: &LlmMaxTokensConfig,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let value = serde_json::to_value(config)
        .map_err(|err| AppError::ConfigError(format!("LLM max_tokens 配置序列化失败: {err}")))?;
    let now = Utc::now().naive_utc();
    let existing = system_settings::Entity::find()
        .filter(system_settings::Column::Key.eq(LLM_MAX_TOKENS_SETTING_KEY))
        .one(db)
        .await
        .map_err(AppError::Database)?;

    if let Some(setting) = existing {
        let mut active: system_settings::ActiveModel = setting.into();
        active.value = Set(value);
        active.category = Set("llm".to_string());
        active.description = Set(Some("LLM 各场景请求输出预算".to_string()));
        active.updated_at = Set(now);
        active.update(db).await.map_err(AppError::Database)?;
    } else {
        system_settings::ActiveModel {
            key: Set(LLM_MAX_TOKENS_SETTING_KEY.to_string()),
            value: Set(value),
            category: Set("llm".to_string()),
            description: Set(Some("LLM 各场景请求输出预算".to_string())),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(AppError::Database)?;
    }

    Ok(())
}
