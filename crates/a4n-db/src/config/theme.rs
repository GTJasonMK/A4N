use a4n_core::{error::AppError, models::user::theme_config};
use a4n_domain::{ThemeCssVariables, ThemeMode};
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
    css_variables: ThemeCssVariables,
) -> Result<theme_config::Model, AppError> {
    let now = Utc::now().naive_utc();
    theme_config::ActiveModel {
        user_id: Set(user_id),
        name: Set(name.to_string()),
        mode: Set(mode.to_string()),
        css_variables: Set(serde_json::to_value(css_variables)?),
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
    css_variables: Option<ThemeCssVariables>,
) -> Result<theme_config::Model, AppError> {
    let mut active: theme_config::ActiveModel = theme.into();
    if let Some(value) = name {
        active.name = Set(value.to_string());
    }
    if let Some(value) = mode {
        active.mode = Set(value.to_string());
    }
    if let Some(value) = css_variables {
        active.css_variables = Set(serde_json::to_value(value)?);
    }
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(AppError::Database)
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
    clear_scope.exec(&txn).await.map_err(AppError::Database)?;

    let mut active: theme_config::ActiveModel = target.into();
    active.is_active = Set(true);
    active.updated_at = Set(now);
    let updated = active.update(&txn).await.map_err(AppError::Database)?;
    txn.commit().await.map_err(AppError::Database)?;
    Ok(updated)
}
