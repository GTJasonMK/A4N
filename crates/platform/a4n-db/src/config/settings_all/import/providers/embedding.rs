use super::super::super::types::{
    SettingsAllConfigImportError, SettingsAllEmbeddingConfigRow, settings_all_config_database,
};
use crate::config::{ProviderConfigKind, clear_provider_config_defaults};
use a4n_db_models::models::ai::embedding_config;
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

pub(in crate::config::settings_all::import) async fn import_embedding_config_row<C>(
    db: &C,
    row: SettingsAllEmbeddingConfigRow,
    overwrite: bool,
    now: NaiveDateTime,
) -> Result<bool, SettingsAllConfigImportError>
where
    C: ConnectionTrait,
{
    let existing = embedding_config::Entity::find()
        .filter(embedding_config::Column::Name.eq(&row.name))
        .one(db)
        .await
        .map_err(|source| settings_all_config_database("读取现有 Embedding 配置失败", source))?;
    if row.is_default && (overwrite || existing.is_none()) {
        clear_provider_config_defaults(db, ProviderConfigKind::Embedding, None)
            .await
            .map_err(|source| {
                settings_all_config_database("清除既有默认 Embedding 配置失败", source)
            })?;
    }

    match (existing, overwrite) {
        (Some(_), false) => Ok(false),
        (Some(existing), true) => {
            let mut active: embedding_config::ActiveModel = existing.into();
            active.backend = Set(row.backend);
            active.base_url = Set(row.base_url);
            active.api_key_encrypted = Set(row.api_key_encrypted);
            active.model = Set(row.model);
            active.dimension = Set(row.dimension);
            active.batch_size = Set(row.batch_size);
            active.local_model_path = Set(row.local_model_path);
            active.pricing_json = Set(row.pricing_json);
            active.is_default = Set(row.is_default);
            active.updated_at = Set(now);
            active.update(db).await.map_err(|source| {
                settings_all_config_database("更新 Embedding 配置失败", source)
            })?;
            Ok(true)
        }
        (None, _) => {
            embedding_config::ActiveModel {
                name: Set(row.name),
                backend: Set(row.backend),
                base_url: Set(row.base_url),
                api_key_encrypted: Set(row.api_key_encrypted),
                model: Set(row.model),
                dimension: Set(row.dimension),
                batch_size: Set(row.batch_size),
                local_model_path: Set(row.local_model_path),
                pricing_json: Set(row.pricing_json),
                is_default: Set(row.is_default),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(|source| settings_all_config_database("插入 Embedding 配置失败", source))?;
            Ok(true)
        }
    }
}
