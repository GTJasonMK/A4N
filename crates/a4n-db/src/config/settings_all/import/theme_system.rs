use super::super::super::LLM_MAX_TOKENS_SETTING_KEY;
use super::super::{
    types::{
        SettingsAllConfigImportError, SettingsAllSystemSettingRow, SettingsAllThemeRow,
        settings_all_config_database,
    },
    validation::validate_llm_max_tokens_import_against_current_default,
};
use a4n_core::models::{ai::system_settings, user::theme_config};
use a4n_domain::validate_theme_document;
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

pub(super) async fn import_theme_row<C>(
    db: &C,
    row: SettingsAllThemeRow,
    overwrite: bool,
    now: NaiveDateTime,
) -> Result<bool, SettingsAllConfigImportError>
where
    C: ConnectionTrait,
{
    let super::super::types::SettingsAllThemeRow {
        user_id,
        name,
        mode,
        document,
        is_system_default,
        is_active,
    } = row;
    let document = document.without_resource_references();
    let mode = mode.to_string();
    validate_theme_document(&document).map_err(SettingsAllConfigImportError::InvalidArgument)?;
    let document_json = serde_json::to_value(&document).map_err(|source| {
        SettingsAllConfigImportError::InvalidArgument(format!("序列化主题 document 失败：{source}"))
    })?;
    let mut query = theme_config::Entity::find().filter(theme_config::Column::Name.eq(&name));
    query = match user_id {
        Some(user_id) => query.filter(theme_config::Column::UserId.eq(user_id)),
        None => query.filter(theme_config::Column::UserId.is_null()),
    };
    let existing = query
        .one(db)
        .await
        .map_err(|source| settings_all_config_database("读取现有主题失败", source))?;
    match (existing, overwrite) {
        (Some(_), false) => Ok(false),
        (Some(existing), true) => {
            let mut active: theme_config::ActiveModel = existing.into();
            active.mode = Set(mode.clone());
            active.theme_document = Set(document_json.clone());
            active.is_system_default = Set(is_system_default);
            active.is_active = Set(is_active);
            active.updated_at = Set(now);
            let updated = active
                .update(db)
                .await
                .map_err(|source| settings_all_config_database("更新主题失败", source))?;
            normalize_theme_activation_flags(db, user_id, updated.id, &mode, is_active).await?;
            normalize_theme_default_flags(db, updated.id, &mode, is_system_default).await?;
            Ok(true)
        }
        (None, _) => {
            let inserted = theme_config::ActiveModel {
                user_id: Set(user_id),
                name: Set(name),
                mode: Set(mode.clone()),
                theme_document: Set(document_json),
                is_system_default: Set(is_system_default),
                is_active: Set(is_active),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(|source| settings_all_config_database("插入主题失败", source))?;
            normalize_theme_activation_flags(db, user_id, inserted.id, &mode, is_active).await?;
            normalize_theme_default_flags(db, inserted.id, &mode, is_system_default).await?;
            Ok(true)
        }
    }
}

pub(super) async fn import_system_setting_row<C>(
    db: &C,
    row: SettingsAllSystemSettingRow,
    overwrite: bool,
    now: NaiveDateTime,
) -> Result<bool, SettingsAllConfigImportError>
where
    C: ConnectionTrait,
{
    if row.key == LLM_MAX_TOKENS_SETTING_KEY {
        validate_llm_max_tokens_import_against_current_default(db, row.value.clone()).await?;
    }
    let existing = system_settings::Entity::find()
        .filter(system_settings::Column::Key.eq(&row.key))
        .one(db)
        .await
        .map_err(|source| settings_all_config_database("读取现有系统设置失败", source))?;

    match (existing, overwrite) {
        (Some(_), false) => Ok(false),
        (Some(existing), true) => {
            let mut active: system_settings::ActiveModel = existing.into();
            active.value = Set(row.value);
            active.category = Set(row.category);
            active.description = Set(row.description);
            active.updated_at = Set(now);
            active
                .update(db)
                .await
                .map_err(|source| settings_all_config_database("更新系统设置失败", source))?;
            Ok(true)
        }
        (None, _) => {
            system_settings::ActiveModel {
                key: Set(row.key),
                value: Set(row.value),
                category: Set(row.category),
                description: Set(row.description),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(|source| settings_all_config_database("插入系统设置失败", source))?;
            Ok(true)
        }
    }
}

async fn normalize_theme_activation_flags<C>(
    db: &C,
    user_id: Option<i64>,
    active_theme_id: i64,
    mode: &str,
    is_active: bool,
) -> Result<(), SettingsAllConfigImportError>
where
    C: ConnectionTrait,
{
    if !is_active {
        return Ok(());
    }

    let mut update = theme_config::Entity::update_many()
        .col_expr(
            theme_config::Column::IsActive,
            sea_orm::sea_query::Expr::value(false),
        )
        .filter(theme_config::Column::IsActive.eq(true))
        .filter(theme_config::Column::Id.ne(active_theme_id));
    update = match user_id {
        Some(user_id) => update.filter(theme_config::Column::UserId.eq(user_id)),
        None => update.filter(theme_config::Column::UserId.is_null()),
    };
    update = update.filter(theme_config::Column::Mode.eq(mode));
    update
        .exec(db)
        .await
        .map_err(|source| settings_all_config_database("归一化主题激活标记失败", source))?;
    Ok(())
}

async fn normalize_theme_default_flags<C>(
    db: &C,
    default_theme_id: i64,
    mode: &str,
    is_system_default: bool,
) -> Result<(), SettingsAllConfigImportError>
where
    C: ConnectionTrait,
{
    if !is_system_default {
        return Ok(());
    }

    theme_config::Entity::update_many()
        .col_expr(
            theme_config::Column::IsSystemDefault,
            sea_orm::sea_query::Expr::value(false),
        )
        .filter(theme_config::Column::IsSystemDefault.eq(true))
        .filter(theme_config::Column::Id.ne(default_theme_id))
        .filter(theme_config::Column::Mode.eq(mode))
        .exec(db)
        .await
        .map_err(|source| settings_all_config_database("归一化系统默认主题标记失败", source))?;
    Ok(())
}
