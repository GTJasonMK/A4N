use super::super::types::{SettingsAllConfigImportError, settings_all_config_database};
use a4n_core::config::app_config::LlmMaxTokensConfig;
use a4n_core::models::ai::llm_config;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};
use serde_json::Value;

pub(in crate::config::settings_all) async fn validate_llm_max_tokens_import_against_current_default<
    C,
>(
    db: &C,
    value: Value,
) -> Result<(), SettingsAllConfigImportError>
where
    C: ConnectionTrait,
{
    let config: LlmMaxTokensConfig = serde_json::from_value(value).map_err(|err| {
        SettingsAllConfigImportError::InvalidArgument(format!("llm_max_tokens 解析失败：{err}"))
    })?;
    config.validate().map_err(|err| {
        SettingsAllConfigImportError::InvalidArgument(format!("llm_max_tokens 无效：{err}"))
    })?;
    let Some(default_model) = llm_config::Entity::find()
        .select_only()
        .column(llm_config::Column::MaxOutputTokens)
        .filter(llm_config::Column::IsDefault.eq(true))
        .into_tuple::<i32>()
        .one(db)
        .await
        .map_err(|source| settings_all_config_database("读取默认 LLM 配置失败", source))?
    else {
        return Ok(());
    };
    let model_cap = usize::try_from(default_model)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            SettingsAllConfigImportError::InvalidArgument(
                "默认 LLM max_output_tokens 必须大于 0".to_string(),
            )
        })?;
    config
        .validate_against_model_output_cap(model_cap)
        .map_err(|err| {
            SettingsAllConfigImportError::InvalidArgument(format!("llm_max_tokens 无效：{err}"))
        })
}
