use super::super::super::types::{
    SettingsAllConfigImportError, SettingsAllVideoConfigRow, settings_all_config_database,
};
use crate::config::{ProviderConfigKind, clear_provider_config_defaults};
use a4n_core::models::ai::video_gen_config;
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

pub(in crate::config::settings_all::import) async fn import_video_config_row<C>(
    db: &C,
    row: SettingsAllVideoConfigRow,
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
    if row.is_default && (overwrite || existing.is_none()) {
        clear_provider_config_defaults(db, ProviderConfigKind::Video, None)
            .await
            .map_err(|source| settings_all_config_database("清除既有默认视频配置失败", source))?;
    }

    match (existing, overwrite) {
        (Some(_), false) => Ok(false),
        (Some(existing), true) => {
            video_active_model_for_update(existing.into(), row, now)
                .update(db)
                .await
                .map_err(|source| settings_all_config_database("更新视频配置失败", source))?;
            Ok(true)
        }
        (None, _) => {
            video_active_model_for_insert(row, now)
                .insert(db)
                .await
                .map_err(|source| settings_all_config_database("插入视频配置失败", source))?;
            Ok(true)
        }
    }
}

fn video_active_model_for_update(
    mut active: video_gen_config::ActiveModel,
    row: SettingsAllVideoConfigRow,
    now: NaiveDateTime,
) -> video_gen_config::ActiveModel {
    active.backend = Set(row.backend);
    active.base_url = Set(Some(row.base_url));
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
    active.is_default = Set(row.is_default);
    active.updated_at = Set(now);
    active
}

fn video_active_model_for_insert(
    row: SettingsAllVideoConfigRow,
    now: NaiveDateTime,
) -> video_gen_config::ActiveModel {
    video_gen_config::ActiveModel {
        name: Set(row.name),
        backend: Set(row.backend),
        base_url: Set(Some(row.base_url)),
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
        is_default: Set(row.is_default),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}
