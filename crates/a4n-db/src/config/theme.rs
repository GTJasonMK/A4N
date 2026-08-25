use a4n_core::{
    error::AppError,
    models::user::{theme_config, theme_preference as theme_preference_entity, theme_resource},
};
use a4n_domain::{ThemeAppearance, ThemeDocument, ThemeMode};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};

pub async fn list_themes_visible_to_user(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<Vec<theme_config::Model>, AppError> {
    theme_config::Entity::find()
        .filter(
            Condition::any()
                .add(theme_config::Column::UserId.eq(user_id))
                .add(theme_config::Column::UserId.is_null()),
        )
        .order_by_asc(theme_config::Column::Name)
        .all(db)
        .await
        .map_err(AppError::Database)
}

pub async fn theme_by_id(
    db: &DatabaseConnection,
    theme_id: i64,
) -> Result<Option<theme_config::Model>, AppError> {
    theme_config::Entity::find_by_id(theme_id)
        .one(db)
        .await
        .map_err(AppError::Database)
}

pub async fn create_theme(
    db: &DatabaseConnection,
    user_id: Option<i64>,
    name: &str,
    mode: ThemeMode,
    document: ThemeDocument,
) -> Result<theme_config::Model, AppError> {
    let now = Utc::now().naive_utc();
    theme_config::ActiveModel {
        user_id: Set(user_id),
        name: Set(name.to_string()),
        mode: Set(mode.to_string()),
        theme_document: Set(serde_json::to_value(document)?),
        is_system_default: Set(false),
        is_active: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}

pub async fn update_theme(
    db: &DatabaseConnection,
    theme: theme_config::Model,
    name: Option<&str>,
    mode: Option<ThemeMode>,
    document: Option<ThemeDocument>,
) -> Result<theme_config::Model, AppError> {
    let next_mode = mode.map(|value| value.to_string());
    let txn = db.begin().await.map_err(AppError::Database)?;
    if theme.is_active
        && next_mode
            .as_deref()
            .is_some_and(|value| value != theme.mode)
    {
        let mut clear_scope = theme_config::Entity::update_many()
            .col_expr(
                theme_config::Column::IsActive,
                sea_orm::sea_query::Expr::value(false),
            )
            .col_expr(
                theme_config::Column::UpdatedAt,
                sea_orm::sea_query::Expr::value(Utc::now().naive_utc()),
            )
            .filter(theme_config::Column::IsActive.eq(true))
            .filter(theme_config::Column::Id.ne(theme.id))
            .filter(theme_config::Column::Mode.eq(next_mode.as_deref().unwrap_or_default()));
        clear_scope = match theme.user_id {
            Some(user_id) => clear_scope.filter(theme_config::Column::UserId.eq(user_id)),
            None => clear_scope.filter(theme_config::Column::UserId.is_null()),
        };
        clear_scope.exec(&txn).await.map_err(AppError::Database)?;
    }
    let mut active: theme_config::ActiveModel = theme.into();
    if let Some(value) = name {
        active.name = Set(value.to_string());
    }
    if let Some(value) = next_mode {
        active.mode = Set(value);
    }
    if let Some(value) = document {
        active.theme_document = Set(serde_json::to_value(value)?);
    }
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(&txn).await.map_err(AppError::Database)?;
    txn.commit().await.map_err(AppError::Database)?;
    Ok(updated)
}

pub async fn delete_theme(db: &DatabaseConnection, theme_id: i64) -> Result<(), AppError> {
    theme_config::Entity::delete_by_id(theme_id)
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn activate_theme(
    db: &DatabaseConnection,
    theme_id: i64,
) -> Result<theme_config::Model, AppError> {
    let txn = db.begin().await.map_err(AppError::Database)?;
    let target = theme_config::Entity::find_by_id(theme_id)
        .one(&txn)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("主题不存在".into()))?;
    let user_id = target.user_id;
    let now = Utc::now().naive_utc();

    let mut clear_scope = theme_config::Entity::update_many()
        .col_expr(
            theme_config::Column::IsActive,
            sea_orm::sea_query::Expr::value(false),
        )
        .col_expr(
            theme_config::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(now),
        );
    clear_scope = match user_id {
        Some(uid) => clear_scope.filter(theme_config::Column::UserId.eq(uid)),
        None => clear_scope.filter(theme_config::Column::UserId.is_null()),
    };
    clear_scope = clear_scope.filter(theme_config::Column::Mode.eq(&target.mode));
    clear_scope.exec(&txn).await.map_err(AppError::Database)?;

    let mut active: theme_config::ActiveModel = target.into();
    active.is_active = Set(true);
    active.updated_at = Set(now);
    let updated = active.update(&txn).await.map_err(AppError::Database)?;
    txn.commit().await.map_err(AppError::Database)?;
    Ok(updated)
}

pub async fn list_theme_resources(
    db: &DatabaseConnection,
    theme_id: i64,
) -> Result<Vec<theme_resource::Model>, AppError> {
    theme_resource::Entity::find()
        .filter(theme_resource::Column::ThemeId.eq(theme_id))
        .order_by_asc(theme_resource::Column::ResourceKey)
        .all(db)
        .await
        .map_err(AppError::Database)
}

pub async fn list_theme_resources_for_themes(
    db: &DatabaseConnection,
    theme_ids: &[i64],
) -> Result<Vec<theme_resource::Model>, AppError> {
    if theme_ids.is_empty() {
        return Ok(Vec::new());
    }
    theme_resource::Entity::find()
        .filter(theme_resource::Column::ThemeId.is_in(theme_ids.to_vec()))
        .order_by_asc(theme_resource::Column::ThemeId)
        .order_by_asc(theme_resource::Column::ResourceKey)
        .all(db)
        .await
        .map_err(AppError::Database)
}

pub async fn theme_resource_by_id(
    db: &DatabaseConnection,
    resource_id: i64,
) -> Result<Option<theme_resource::Model>, AppError> {
    theme_resource::Entity::find_by_id(resource_id)
        .one(db)
        .await
        .map_err(AppError::Database)
}

pub async fn upsert_theme_resource(
    db: &DatabaseConnection,
    theme_id: i64,
    resource_key: &str,
    file_name: &str,
    mime_type: &str,
    file_path: &str,
    sha256: &str,
    byte_size: i64,
) -> Result<theme_resource::Model, AppError> {
    let existing = theme_resource::Entity::find()
        .filter(theme_resource::Column::ThemeId.eq(theme_id))
        .filter(theme_resource::Column::ResourceKey.eq(resource_key))
        .one(db)
        .await
        .map_err(AppError::Database)?;
    let now = Utc::now().naive_utc();
    if let Some(existing) = existing {
        let mut active: theme_resource::ActiveModel = existing.into();
        active.file_name = Set(file_name.to_string());
        active.mime_type = Set(mime_type.to_string());
        active.file_path = Set(file_path.to_string());
        active.sha256 = Set(sha256.to_string());
        active.byte_size = Set(byte_size);
        active.updated_at = Set(now);
        return active.update(db).await.map_err(AppError::Database);
    }
    theme_resource::ActiveModel {
        theme_id: Set(theme_id),
        resource_key: Set(resource_key.to_string()),
        file_name: Set(file_name.to_string()),
        mime_type: Set(mime_type.to_string()),
        file_path: Set(file_path.to_string()),
        sha256: Set(sha256.to_string()),
        byte_size: Set(byte_size),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}

pub async fn delete_theme_resource(
    db: &DatabaseConnection,
    resource_id: i64,
) -> Result<theme_resource::Model, AppError> {
    let resource = theme_resource_by_id(db, resource_id)
        .await?
        .ok_or_else(|| AppError::NotFound("主题资源不存在".to_string()))?;
    theme_resource::Entity::delete_by_id(resource_id)
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    Ok(resource)
}

pub async fn theme_preference(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<Option<theme_preference_entity::Model>, AppError> {
    theme_preference_entity::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(AppError::Database)
}

pub async fn upsert_theme_preference(
    db: &DatabaseConnection,
    user_id: i64,
    appearance: ThemeAppearance,
) -> Result<theme_preference_entity::Model, AppError> {
    let now = Utc::now().naive_utc();
    if let Some(existing) = theme_preference(db, user_id).await? {
        let mut active: theme_preference_entity::ActiveModel = existing.into();
        active.appearance = Set(appearance.to_string());
        active.updated_at = Set(now);
        return active.update(db).await.map_err(AppError::Database);
    }
    theme_preference_entity::ActiveModel {
        user_id: Set(user_id),
        appearance: Set(appearance.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}

#[cfg(test)]
mod tests {
    use super::{create_theme, list_theme_resources_for_themes, upsert_theme_resource};
    use a4n_domain::{ThemeDocument, ThemeMode};

    #[tokio::test]
    async fn list_theme_resources_for_themes_returns_grouped_rows_in_order() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let light = create_theme(
            &db,
            None,
            "批量主题浅色",
            ThemeMode::Light,
            ThemeDocument::default(),
        )
        .await
        .unwrap();
        let dark = create_theme(
            &db,
            None,
            "批量主题深色",
            ThemeMode::Dark,
            ThemeDocument::default(),
        )
        .await
        .unwrap();
        upsert_theme_resource(
            &db,
            light.id,
            "settings",
            "settings.png",
            "image/png",
            "themes/light-settings.png",
            "light-settings-hash",
            2,
        )
        .await
        .unwrap();
        upsert_theme_resource(
            &db,
            dark.id,
            "home",
            "home.png",
            "image/png",
            "themes/dark-home.png",
            "dark-home-hash",
            1,
        )
        .await
        .unwrap();

        let rows = list_theme_resources_for_themes(&db, &[dark.id, light.id])
            .await
            .unwrap();

        assert_eq!(
            rows.iter().map(|row| row.theme_id).collect::<Vec<_>>(),
            vec![light.id, dark.id]
        );
        assert_eq!(rows[0].resource_key, "settings");
        assert_eq!(rows[1].resource_key, "home");
        assert!(
            list_theme_resources_for_themes(&db, &[])
                .await
                .unwrap()
                .is_empty()
        );
    }
}
