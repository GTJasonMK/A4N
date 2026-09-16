use super::super::super::types::{
    SettingsAllConfigImportError, SettingsAllLlmConfigRow, settings_all_config_database,
};
use crate::config::{ProviderConfigKind, clear_provider_config_defaults};
use a4n_db_models::models::ai::llm_config;
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

pub(in crate::config::settings_all::import) async fn import_llm_config_row<C>(
    db: &C,
    row: SettingsAllLlmConfigRow,
    overwrite: bool,
    now: NaiveDateTime,
) -> Result<bool, SettingsAllConfigImportError>
where
    C: ConnectionTrait,
{
    let existing = llm_config::Entity::find()
        .filter(llm_config::Column::Name.eq(&row.name))
        .one(db)
        .await
        .map_err(|source| settings_all_config_database("读取现有 LLM 配置失败", source))?;
    if row.is_default && (overwrite || existing.is_none()) {
        clear_provider_config_defaults(db, ProviderConfigKind::Llm, None)
            .await
            .map_err(|source| settings_all_config_database("清除既有默认 LLM 配置失败", source))?;
    }

    match (existing, overwrite) {
        (Some(_), false) => Ok(false),
        (Some(existing), true) => {
            let mut active: llm_config::ActiveModel = existing.into();
            active.provider = Set(row.provider);
            active.base_url = Set(row.base_url);
            active.api_key_encrypted = Set(row.api_key_encrypted);
            active.model = Set(row.model);
            active.max_concurrent = Set(row.max_concurrent);
            active.timeout_secs = Set(row.timeout_secs);
            active.max_context_tokens = Set(row.max_context_tokens);
            active.max_output_tokens = Set(row.max_output_tokens);
            active.extra_body_json = Set(row.extra_body_json);
            active.pricing_json = Set(row.pricing_json);
            active.is_default = Set(row.is_default);
            active.updated_at = Set(now);
            active
                .update(db)
                .await
                .map_err(|source| settings_all_config_database("更新 LLM 配置失败", source))?;
            Ok(true)
        }
        (None, _) => {
            llm_config::ActiveModel {
                name: Set(row.name),
                provider: Set(row.provider),
                base_url: Set(row.base_url),
                api_key_encrypted: Set(row.api_key_encrypted),
                model: Set(row.model),
                max_concurrent: Set(row.max_concurrent),
                timeout_secs: Set(row.timeout_secs),
                max_context_tokens: Set(row.max_context_tokens),
                max_output_tokens: Set(row.max_output_tokens),
                extra_body_json: Set(row.extra_body_json),
                pricing_json: Set(row.pricing_json),
                is_default: Set(row.is_default),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(|source| settings_all_config_database("插入 LLM 配置失败", source))?;
            Ok(true)
        }
    }
}
