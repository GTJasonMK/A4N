use super::super::super::LLM_MAX_TOKENS_SETTING_KEY;
use super::super::{
    types::{
        SettingsAllConfigImportError, SettingsAllSystemSettingImportRow, SettingsAllThemeImportRow,
        settings_all_config_database,
    },
    validation::validate_llm_max_tokens_import_against_current_default,
};
use a4n_core::models::{ai::system_settings, user::theme_config};
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

pub(super) async fn import_theme_row<C>(
    db: &C,
    row: SettingsAllThemeImportRow,
    overwrite: bool,
    now: NaiveDateTime,
) -> Result<bool, SettingsAllConfigImportError>
where
    C: ConnectionTrait,
{
    let mut query = theme_config::Entity::find().filter(theme_config::Column::Name.eq(&row.name));
    query = match row.user_id {
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
            active.mode = Set(row.mode);
            active.css_variables = Set(row.css_variables);
            active.is_system_default = Set(row.is_system_default);
            active.is_active = Set(row.is_active);
            active.updated_at = Set(now);
            let updated = active
                .update(db)
                .await
                .map_err(|source| settings_all_config_database("更新主题失败", source))?;
            normalize_theme_activation_flags(db, row.user_id, updated.id, row.is_active).await?;
            normalize_theme_default_flags(db, updated.id, row.is_system_default).await?;
            Ok(true)
        }
        (None, _) => {
            let inserted = theme_config::ActiveModel {
                user_id: Set(row.user_id),
                name: Set(row.name),
                mode: Set(row.mode),
                css_variables: Set(row.css_variables),
                is_system_default: Set(row.is_system_default),
                is_active: Set(row.is_active),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(|source| settings_all_config_database("插入主题失败", source))?;
            normalize_theme_activation_flags(db, row.user_id, inserted.id, row.is_active).await?;
            normalize_theme_default_flags(db, inserted.id, row.is_system_default).await?;
            Ok(true)
        }
    }
}

pub(super) async fn import_system_setting_row<C>(
    db: &C,
    row: SettingsAllSystemSettingImportRow,
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
    update
        .exec(db)
        .await
        .map_err(|source| settings_all_config_database("归一化主题激活标记失败", source))?;
    Ok(())
}

async fn normalize_theme_default_flags<C>(
    db: &C,
    default_theme_id: i64,
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
        .exec(db)
        .await
        .map_err(|source| settings_all_config_database("归一化系统默认主题标记失败", source))?;
    Ok(())
}
