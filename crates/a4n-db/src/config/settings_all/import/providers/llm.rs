use super::super::super::types::{
    SettingsAllConfigImportError, SettingsAllLlmConfigImportRow, settings_all_config_database,
};
use super::defaults::clear_llm_defaults;
use a4n_core::models::ai::llm_config;
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

pub(in crate::config::settings_all::import) async fn import_llm_config_row<C>(
    db: &C,
    row: SettingsAllLlmConfigImportRow,
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

    match (existing, overwrite) {
        (Some(_), false) => Ok(false),
        (Some(existing), true) => {
            if row.is_default {
                clear_llm_defaults(db).await?;
            }
            let mut active: llm_config::ActiveModel = existing.into();
            active.provider = Set(row.provider);
            active.base_url = Set(row.base_url);
            active.api_key_encrypted = Set(row.api_key_encrypted);
            active.model = Set(row.model);
            active.max_concurrent = Set(row.max_concurrent);
            active.timeout_secs = Set(row.timeout_secs);
            active.max_context_tokens = Set(row.max_context_tokens);
            active.max_output_tokens = Set(row.max_output_tokens);
            active.request_options_json = Set(row.request_options_json);
            active.is_default = Set(row.is_default);
            active.updated_at = Set(now);
            active
                .update(db)
                .await
                .map_err(|source| settings_all_config_database("更新 LLM 配置失败", source))?;
            Ok(true)
        }
        (None, _) => {
            if row.is_default {
                clear_llm_defaults(db).await?;
            }
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
                request_options_json: Set(row.request_options_json),
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
