use a4n_db_models::models::user::{
    theme_config, theme_preference as theme_preference_entity, theme_resource,
};
use a4n_domain::theme_edit::{
    ThemePreferenceRecord, ThemeRecord, ThemeResourceRecord, ThemeUpdateChanges,
};
use a4n_domain::{ThemeAppearance, ThemeDocument, ThemeMode};
use a4n_error::AppError;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};

pub async fn list_themes_visible_to_user(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<Vec<ThemeRecord>, AppError> {
    theme_config::Entity::find()
        .filter(
            Condition::any()
                .add(theme_config::Column::UserId.eq(user_id))
                .add(theme_config::Column::UserId.is_null()),
        )
        .order_by_asc(theme_config::Column::Name)
        .all(db)
        .await
        .map(|rows| rows.into_iter().map(theme_record).collect())
        .map_err(AppError::database)
}

pub async fn theme_by_id(
    db: &DatabaseConnection,
    theme_id: i64,
) -> Result<Option<ThemeRecord>, AppError> {
    theme_config::Entity::find_by_id(theme_id)
        .one(db)
        .await
        .map(|row| row.map(theme_record))
        .map_err(AppError::database)
}

pub async fn create_theme(
    db: &DatabaseConnection,
    user_id: Option<i64>,
    name: &str,
    mode: ThemeMode,
    document: ThemeDocument,
) -> Result<ThemeRecord, AppError> {
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
    .map(theme_record)
    .map_err(AppError::database)
}

pub async fn update_theme(
    db: &DatabaseConnection,
    theme: ThemeRecord,
    changes: ThemeUpdateChanges,
) -> Result<(ThemeRecord, Vec<ThemeResourceRecord>), AppError> {
    let txn = db.begin().await.map_err(AppError::database)?;
    let current = theme_config::Entity::find_by_id(theme.id)
        .lock_exclusive()
        .one(&txn)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::NotFound("主题不存在".into()))?;
    if current.user_id != theme.user_id {
        return Err(AppError::Forbidden("主题所有者已变化".into()));
    }
    let theme = current;
    let next_mode = changes.mode.map(|value| value.to_string());
    if theme.is_active
        && let Some(mode) = next_mode.as_deref().filter(|mode| *mode != theme.mode)
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
            .filter(theme_config::Column::Mode.eq(mode));
        clear_scope = match theme.user_id {
            Some(user_id) => clear_scope.filter(theme_config::Column::UserId.eq(user_id)),
            None => clear_scope.filter(theme_config::Column::UserId.is_null()),
        };
        clear_scope.exec(&txn).await.map_err(AppError::database)?;
    }
    let theme_id = theme.id;
    let mut active: theme_config::ActiveModel = theme.into();
    if let Some(value) = changes.name {
        active.name = Set(value);
    }
    if let Some(value) = next_mode {
        active.mode = Set(value);
    }
    if let Some(value) = changes.document {
        active.theme_document = Set(serde_json::to_value(value)?);
    }
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(&txn).await.map_err(AppError::database)?;
    let mut keys = std::collections::HashSet::new();
    for resource in changes.resources {
        if resource.theme_id != theme_id || !keys.insert(resource.resource_key.clone()) {
            return Err(AppError::InvalidArgument(
                "主题资源归属不一致或资源键重复".into(),
            ));
        }
        upsert_theme_resource(
            &txn,
            theme_id,
            &resource.resource_key,
            &resource.file_name,
            &resource.mime_type,
            &resource.file_path,
            &resource.sha256,
            resource.byte_size,
        )
        .await?;
    }
    let deleted_ids = changes
        .delete_resource_ids
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
    if !deleted_ids.is_empty() {
        let deleted = theme_resource::Entity::delete_many()
            .filter(theme_resource::Column::ThemeId.eq(theme_id))
            .filter(theme_resource::Column::Id.is_in(deleted_ids.iter().copied()))
            .exec(&txn)
            .await
            .map_err(AppError::database)?;
        if deleted.rows_affected != deleted_ids.len() as u64 {
            return Err(AppError::InvalidArgument(
                "待删除主题资源不存在或不属于该主题".into(),
            ));
        }
    }
    let resources = list_theme_resources(&txn, theme_id).await?;
    let document: ThemeDocument = serde_json::from_value(updated.theme_document.clone())?;
    for background in document.backgrounds.values() {
        if let Some(key) = &background.resource_key
            && !resources
                .iter()
                .any(|resource| &resource.resource_key == key)
        {
            return Err(AppError::InvalidArgument(format!(
                "主题背景引用了不存在的资源: {key}"
            )));
        }
    }
    txn.commit().await.map_err(AppError::database)?;
    Ok((theme_record(updated), resources))
}

pub async fn delete_theme(db: &DatabaseConnection, theme_id: i64) -> Result<(), AppError> {
    theme_config::Entity::delete_by_id(theme_id)
        .exec(db)
        .await
        .map_err(AppError::database)?;
    Ok(())
}

pub async fn activate_theme(
    db: &DatabaseConnection,
    theme_id: i64,
) -> Result<ThemeRecord, AppError> {
    let txn = db.begin().await.map_err(AppError::database)?;
    let target = theme_config::Entity::find_by_id(theme_id)
        .one(&txn)
        .await
        .map_err(AppError::database)?
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
    clear_scope.exec(&txn).await.map_err(AppError::database)?;

    let mut active: theme_config::ActiveModel = target.into();
    active.is_active = Set(true);
    active.updated_at = Set(now);
    let updated = active.update(&txn).await.map_err(AppError::database)?;
    txn.commit().await.map_err(AppError::database)?;
    Ok(theme_record(updated))
}

pub async fn list_theme_resources(
    db: &impl ConnectionTrait,
    theme_id: i64,
) -> Result<Vec<ThemeResourceRecord>, AppError> {
    theme_resource::Entity::find()
        .filter(theme_resource::Column::ThemeId.eq(theme_id))
        .order_by_asc(theme_resource::Column::ResourceKey)
        .all(db)
        .await
        .map(|rows| rows.into_iter().map(theme_resource_record).collect())
        .map_err(AppError::database)
}

pub async fn list_theme_resources_for_themes(
    db: &DatabaseConnection,
    theme_ids: &[i64],
) -> Result<Vec<ThemeResourceRecord>, AppError> {
    if theme_ids.is_empty() {
        return Ok(Vec::new());
    }
    theme_resource::Entity::find()
        .filter(theme_resource::Column::ThemeId.is_in(theme_ids.to_vec()))
        .order_by_asc(theme_resource::Column::ThemeId)
        .order_by_asc(theme_resource::Column::ResourceKey)
        .all(db)
        .await
        .map(|rows| rows.into_iter().map(theme_resource_record).collect())
        .map_err(AppError::database)
}

pub async fn theme_resource_by_id(
    db: &DatabaseConnection,
    resource_id: i64,
) -> Result<Option<ThemeResourceRecord>, AppError> {
    theme_resource::Entity::find_by_id(resource_id)
        .one(db)
        .await
        .map(|row| row.map(theme_resource_record))
        .map_err(AppError::database)
}

pub async fn upsert_theme_resource(
    db: &impl ConnectionTrait,
    theme_id: i64,
    resource_key: &str,
    file_name: &str,
    mime_type: &str,
    file_path: &str,
    sha256: &str,
    byte_size: i64,
) -> Result<ThemeResourceRecord, AppError> {
    let existing = theme_resource::Entity::find()
        .filter(theme_resource::Column::ThemeId.eq(theme_id))
        .filter(theme_resource::Column::ResourceKey.eq(resource_key))
        .one(db)
        .await
        .map_err(AppError::database)?;
    let now = Utc::now().naive_utc();
    if let Some(existing) = existing {
        let mut active: theme_resource::ActiveModel = existing.into();
        active.file_name = Set(file_name.to_string());
        active.mime_type = Set(mime_type.to_string());
        active.file_path = Set(file_path.to_string());
        active.sha256 = Set(sha256.to_string());
        active.byte_size = Set(byte_size);
        active.updated_at = Set(now);
        return active
            .update(db)
            .await
            .map(theme_resource_record)
            .map_err(AppError::database);
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
    .map(theme_resource_record)
    .map_err(AppError::database)
}

pub async fn delete_theme_resource(
    db: &DatabaseConnection,
    resource_id: i64,
) -> Result<ThemeResourceRecord, AppError> {
    let resource = theme_resource_by_id(db, resource_id)
        .await?
        .ok_or_else(|| AppError::NotFound("主题资源不存在".to_string()))?;
    theme_resource::Entity::delete_by_id(resource_id)
        .exec(db)
        .await
        .map_err(AppError::database)?;
    Ok(resource)
}

pub async fn theme_preference(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<Option<ThemePreferenceRecord>, AppError> {
    theme_preference_entity::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map(|row| row.map(theme_preference_record))
        .map_err(AppError::database)
}

pub async fn upsert_theme_preference(
    db: &DatabaseConnection,
    user_id: i64,
    appearance: ThemeAppearance,
) -> Result<ThemePreferenceRecord, AppError> {
    let now = Utc::now().naive_utc();
    if let Some(existing) = theme_preference_entity::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(AppError::database)?
    {
        let mut active: theme_preference_entity::ActiveModel = existing.into();
        active.appearance = Set(appearance.to_string());
        active.updated_at = Set(now);
        return active
            .update(db)
            .await
            .map(theme_preference_record)
            .map_err(AppError::database);
    }
    theme_preference_entity::ActiveModel {
        user_id: Set(user_id),
        appearance: Set(appearance.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .map(theme_preference_record)
    .map_err(AppError::database)
}

fn theme_record(row: theme_config::Model) -> ThemeRecord {
    ThemeRecord {
        id: row.id,
        user_id: row.user_id,
        name: row.name,
        mode: row.mode,
        theme_document: row.theme_document,
        is_system_default: row.is_system_default,
        is_active: row.is_active,
    }
}

fn theme_resource_record(row: theme_resource::Model) -> ThemeResourceRecord {
    ThemeResourceRecord {
        id: row.id,
        theme_id: row.theme_id,
        resource_key: row.resource_key,
        file_name: row.file_name,
        mime_type: row.mime_type,
        file_path: row.file_path,
        sha256: row.sha256,
        byte_size: row.byte_size,
    }
}

fn theme_preference_record(row: theme_preference_entity::Model) -> ThemePreferenceRecord {
    ThemePreferenceRecord {
        user_id: row.user_id,
        appearance: row.appearance,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        create_theme, list_theme_resources, list_theme_resources_for_themes, theme_by_id,
        update_theme, upsert_theme_resource,
    };
    use a4n_domain::{ThemeDocument, ThemeMode};

    #[tokio::test]
    async fn theme_document_and_resource_changes_commit_together() {
        use a4n_domain::theme_edit::{PersistThemeResourceInput, ThemeUpdateChanges};
        let db = a4n_migration::test_support::connect_test_database().await;
        let theme = create_theme(
            &db,
            None,
            "原主题",
            ThemeMode::Light,
            ThemeDocument::default(),
        )
        .await
        .unwrap();
        let resource = upsert_theme_resource(
            &db,
            theme.id,
            "global",
            "red.png",
            "image/png",
            "themes/red.png",
            "red",
            10,
        )
        .await
        .unwrap();
        let replacement = PersistThemeResourceInput {
            theme_id: theme.id,
            resource_key: "global".into(),
            file_name: "blue.png".into(),
            mime_type: "image/png".into(),
            file_path: "themes/blue.png".into(),
            sha256: "blue".into(),
            byte_size: 20,
        };
        let mut document = ThemeDocument::default();
        document.backgrounds.insert(
            "global".into(),
            a4n_domain::ThemeBackgroundSettings {
                resource_key: Some("global".into()),
                ..Default::default()
            },
        );
        let error = update_theme(
            &db,
            theme.clone(),
            ThemeUpdateChanges {
                name: Some("不应保存".into()),
                mode: None,
                document: Some(document.clone()),
                resources: vec![replacement.clone()],
                delete_resource_ids: vec![i64::MAX],
            },
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("待删除主题资源"));
        assert_eq!(theme_by_id(&db, theme.id).await.unwrap().unwrap(), theme);
        assert_eq!(
            list_theme_resources(&db, theme.id).await.unwrap(),
            vec![resource.clone()]
        );
        let (saved, resources) = update_theme(
            &db,
            theme.clone(),
            ThemeUpdateChanges {
                name: Some("新主题".into()),
                mode: None,
                document: Some(document.clone()),
                resources: vec![replacement],
                delete_resource_ids: vec![],
            },
        )
        .await
        .unwrap();
        assert_eq!(saved.name, "新主题");
        assert_eq!(resources[0].id, resource.id);
        assert_eq!(resources[0].file_path, "themes/blue.png");
        assert!(
            update_theme(
                &db,
                saved.clone(),
                ThemeUpdateChanges {
                    name: None,
                    mode: None,
                    document: None,
                    resources: vec![],
                    delete_resource_ids: vec![resource.id],
                }
            )
            .await
            .unwrap_err()
            .to_string()
            .contains("引用了不存在的资源")
        );
        assert_eq!(
            list_theme_resources(&db, theme.id).await.unwrap(),
            resources
        );
        document.backgrounds.get_mut("global").unwrap().resource_key = None;
        let (_, resources) = update_theme(
            &db,
            saved,
            ThemeUpdateChanges {
                name: None,
                mode: None,
                document: Some(document),
                resources: vec![],
                delete_resource_ids: vec![resource.id],
            },
        )
        .await
        .unwrap();
        assert!(resources.is_empty());
    }

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
