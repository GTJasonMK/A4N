use super::super::super::types::{
    SettingsAllConfigImportError, SettingsAllImageConfigImportRow, settings_all_config_database,
};
use super::defaults::clear_image_defaults;
use a4n_core::models::ai::image_gen_config;
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

pub(in crate::config::settings_all::import) async fn import_image_config_row<C>(
    db: &C,
    row: SettingsAllImageConfigImportRow,
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

    match (existing, overwrite) {
        (Some(_), false) => Ok(false),
        (Some(existing), true) => {
            if row.is_default {
                clear_image_defaults(db).await?;
            }
            image_active_model_for_update(existing.into(), row, now)
                .update(db)
                .await
                .map_err(|source| settings_all_config_database("更新图片配置失败", source))?;
            Ok(true)
        }
        (None, _) => {
            if row.is_default {
                clear_image_defaults(db).await?;
            }
            image_active_model_for_insert(row, now)
                .insert(db)
                .await
                .map_err(|source| settings_all_config_database("插入图片配置失败", source))?;
            Ok(true)
        }
    }
}

pub(in crate::config::settings_all) fn image_active_model_for_update(
    mut active: image_gen_config::ActiveModel,
    row: SettingsAllImageConfigImportRow,
    now: NaiveDateTime,
) -> image_gen_config::ActiveModel {
    active.backend = Set(row.backend);
    active.base_url = Set(row.base_url);
    active.api_key_encrypted = Set(row.api_key_encrypted);
    active.model = Set(row.model);
    active.max_concurrent = Set(row.max_concurrent);
    active.default_width = Set(row.default_width);
    active.default_height = Set(row.default_height);
    active.provider_options_json = Set(row.provider_options_json);
    active.is_default = Set(row.is_default);
    active.updated_at = Set(now);
    active
}

pub(in crate::config::settings_all) fn image_active_model_for_insert(
    row: SettingsAllImageConfigImportRow,
    now: NaiveDateTime,
) -> image_gen_config::ActiveModel {
    image_gen_config::ActiveModel {
        name: Set(row.name),
        backend: Set(row.backend),
        base_url: Set(row.base_url),
        api_key_encrypted: Set(row.api_key_encrypted),
        model: Set(row.model),
        max_concurrent: Set(row.max_concurrent),
        default_width: Set(row.default_width),
        default_height: Set(row.default_height),
        provider_options_json: Set(row.provider_options_json),
        is_default: Set(row.is_default),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}
