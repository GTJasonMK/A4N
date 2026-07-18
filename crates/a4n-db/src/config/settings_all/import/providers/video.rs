use super::super::super::types::{
    SettingsAllConfigImportError, SettingsAllVideoConfigImportRow, settings_all_config_database,
};
use super::defaults::clear_video_defaults;
use a4n_core::models::ai::video_gen_config;
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

pub(in crate::config::settings_all::import) async fn import_video_config_row<C>(
    db: &C,
    row: SettingsAllVideoConfigImportRow,
    overwrite: bool,
    now: NaiveDateTime,
) -> Result<bool, SettingsAllConfigImportError>
where
    C: ConnectionTrait,
{
    let existing = video_gen_config::Entity::find()
        .filter(video_gen_config::Column::Name.eq(&row.name))
        .one(db)
        .await
        .map_err(|source| settings_all_config_database("读取现有视频配置失败", source))?;

    match (existing, overwrite) {
        (Some(_), false) => Ok(false),
        (Some(existing), true) => {
            if row.is_default {
                clear_video_defaults(db).await?;
            }
            video_active_model_for_update(existing.into(), row, now)
                .update(db)
                .await
                .map_err(|source| settings_all_config_database("更新视频配置失败", source))?;
            Ok(true)
        }
        (None, _) => {
            if row.is_default {
                clear_video_defaults(db).await?;
            }
            video_active_model_for_insert(row, now)
                .insert(db)
                .await
                .map_err(|source| settings_all_config_database("插入视频配置失败", source))?;
            Ok(true)
        }
    }
}

pub(in crate::config::settings_all) fn video_active_model_for_update(
    mut active: video_gen_config::ActiveModel,
    row: SettingsAllVideoConfigImportRow,
    now: NaiveDateTime,
) -> video_gen_config::ActiveModel {
    active.backend = Set(row.backend);
    active.base_url = Set(row.base_url);
    active.api_key_encrypted = Set(row.api_key_encrypted);
    active.model = Set(row.model);
    active.max_concurrent = Set(row.max_concurrent);
    active.default_width = Set(row.default_width);
    active.default_height = Set(row.default_height);
    active.default_duration_secs = Set(row.default_duration_secs);
    active.min_duration_secs = Set(row.min_duration_secs);
    active.max_duration_secs = Set(row.max_duration_secs);
    active.duration_mode = Set(row.duration_mode);
    active.supported_durations_json = Set(row.supported_durations_json);
    active.default_fps = Set(row.default_fps);
    active.provider_options_json = Set(row.provider_options_json);
    active.is_default = Set(row.is_default);
    active.updated_at = Set(now);
    active
}

pub(in crate::config::settings_all) fn video_active_model_for_insert(
    row: SettingsAllVideoConfigImportRow,
    now: NaiveDateTime,
) -> video_gen_config::ActiveModel {
    video_gen_config::ActiveModel {
        name: Set(row.name),
        backend: Set(row.backend),
        base_url: Set(row.base_url),
        api_key_encrypted: Set(row.api_key_encrypted),
        model: Set(row.model),
        max_concurrent: Set(row.max_concurrent),
        default_width: Set(row.default_width),
        default_height: Set(row.default_height),
        default_duration_secs: Set(row.default_duration_secs),
        min_duration_secs: Set(row.min_duration_secs),
        max_duration_secs: Set(row.max_duration_secs),
        duration_mode: Set(row.duration_mode),
        supported_durations_json: Set(row.supported_durations_json),
        default_fps: Set(row.default_fps),
        provider_options_json: Set(row.provider_options_json),
        is_default: Set(row.is_default),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}
