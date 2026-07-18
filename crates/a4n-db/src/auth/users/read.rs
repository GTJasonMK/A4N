use a4n_core::error::AppError;
use a4n_core::models::user::user;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};

pub async fn user_by_id(db: &DatabaseConnection, user_id: i64) -> Result<user::Model, AppError> {
    user::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("用户不存在: {user_id}")))
}

pub async fn current_user(db: &DatabaseConnection, user_id: i64) -> Result<user::Model, AppError> {
    active_user(db, user_id).await
}

pub async fn count_users(db: &DatabaseConnection) -> Result<u64, AppError> {
    user::Entity::find()
        .count(db)
        .await
        .map_err(AppError::Database)
}

pub async fn list_users(db: &DatabaseConnection) -> Result<Vec<user::Model>, AppError> {
    user::Entity::find()
        .order_by_asc(user::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)
}

pub(in crate::auth) async fn active_user(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<user::Model, AppError> {
    let user = user_by_id(db, user_id).await?;
    if !user.is_active {
        return Err(AppError::Forbidden("账户已被禁用".into()));
    }
    Ok(user)
}

pub(in crate::auth) async fn find_by_username_on<C>(
    db: &C,
    username: &str,
) -> Result<Option<user::Model>, AppError>
where
    C: ConnectionTrait,
{
    user::Entity::find()
        .filter(user::Column::Username.eq(username))
        .one(db)
        .await
        .map_err(AppError::Database)
}

pub(in crate::auth) async fn find_by_email_on<C>(
    db: &C,
    email: &str,
) -> Result<Option<user::Model>, AppError>
where
    C: ConnectionTrait,
{
    user::Entity::find()
        .filter(user::Column::Email.eq(email))
        .one(db)
        .await
        .map_err(AppError::Database)
}
