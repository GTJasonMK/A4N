use super::super::super::types::{
    SettingsAllConfigImportError, SettingsAllImageConfigRow, settings_all_config_database,
};
use crate::config::{ProviderConfigKind, clear_provider_config_defaults};
use a4n_core::config::image_sizes_to_json;
use a4n_core::models::ai::image_gen_config;
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

pub(in crate::config::settings_all::import) async fn import_image_config_row<C>(
    db: &C,
    row: SettingsAllImageConfigRow,
    overwrite: bool,
    now: NaiveDateTime,
) -> Result<bool, SettingsAllConfigImportError>
where
    C: ConnectionTrait,
{
    let existing = image_gen_config::Entity::find()
        .filter(image_gen_config::Column::Name.eq(&row.name))
        .one(db)
        .await
        .map_err(|source| settings_all_config_database("读取现有图片配置失败", source))?;
    if row.is_default && (overwrite || existing.is_none()) {
        clear_provider_config_defaults(db, ProviderConfigKind::Image, None)
            .await
            .map_err(|source| settings_all_config_database("清除既有默认图片配置失败", source))?;
    }

    match (existing, overwrite) {
        (Some(_), false) => Ok(false),
        (Some(existing), true) => {
            image_active_model_for_update(existing.into(), row, now)
                .update(db)
                .await
                .map_err(|source| settings_all_config_database("更新图片配置失败", source))?;
            Ok(true)
        }
        (None, _) => {
            image_active_model_for_insert(row, now)
                .insert(db)
                .await
                .map_err(|source| settings_all_config_database("插入图片配置失败", source))?;
            Ok(true)
        }
    }
}

fn image_active_model_for_update(
    mut active: image_gen_config::ActiveModel,
    row: SettingsAllImageConfigRow,
    now: NaiveDateTime,
) -> image_gen_config::ActiveModel {
    active.backend = Set(row.backend);
    active.base_url = Set(Some(row.base_url));
    active.api_key_encrypted = Set(row.api_key_encrypted);
    active.model = Set(row.model);
    active.max_concurrent = Set(row.max_concurrent);
    active.supported_sizes_json = Set(image_sizes_to_json(&row.supported_sizes)
        .expect("settings-all 图片尺寸已在导入校验阶段验证"));
    active.is_default = Set(row.is_default);
    active.updated_at = Set(now);
    active
}

fn image_active_model_for_insert(
    row: SettingsAllImageConfigRow,
    now: NaiveDateTime,
) -> image_gen_config::ActiveModel {
    image_gen_config::ActiveModel {
        name: Set(row.name),
        backend: Set(row.backend),
        base_url: Set(Some(row.base_url)),
        api_key_encrypted: Set(row.api_key_encrypted),
        model: Set(row.model),
        max_concurrent: Set(row.max_concurrent),
        supported_sizes_json: Set(image_sizes_to_json(&row.supported_sizes)
            .expect("settings-all 图片尺寸已在导入校验阶段验证")),
        is_default: Set(row.is_default),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}
