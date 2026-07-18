use std::sync::Arc;

use crate::SharedDatabaseConnection;
use a4n_core::models::user::{invite_code, user, user_session};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};

pub(super) async fn setup() -> SharedDatabaseConnection {
    let db = a4n_migration::test_support::connect_test_database().await;
    Arc::new(db)
}

pub(super) async fn deactivate_user(db: &SharedDatabaseConnection, user_id: i64) {
    set_user_active(db, user_id, false).await;
}

pub(super) async fn set_user_active(db: &SharedDatabaseConnection, user_id: i64, is_active: bool) {
    let user = crate::auth::user_by_id(db.as_ref(), user_id).await.unwrap();
    let mut active: user::ActiveModel = user.into();
    active.is_active = Set(is_active);
    active.update(db.as_ref()).await.unwrap();
}

pub(super) async fn user_count(db: &SharedDatabaseConnection) -> u64 {
    user::Entity::find().count(db.as_ref()).await.unwrap()
}

pub(super) async fn find_user_by_username(
    db: &SharedDatabaseConnection,
    username: &str,
) -> Option<user::Model> {
    user::Entity::find()
        .filter(user::Column::Username.eq(username))
        .one(db.as_ref())
        .await
        .unwrap()
}

pub(super) async fn find_user_by_email(
    db: &SharedDatabaseConnection,
    email: &str,
) -> Option<user::Model> {
    user::Entity::find()
        .filter(user::Column::Email.eq(email))
        .one(db.as_ref())
        .await
        .unwrap()
}

pub(super) async fn session_count(db: &SharedDatabaseConnection) -> u64 {
    user_session::Entity::find()
        .count(db.as_ref())
        .await
        .unwrap()
}

pub(super) async fn invite_code_count(db: &SharedDatabaseConnection) -> u64 {
    invite_code::Entity::find()
        .count(db.as_ref())
        .await
        .unwrap()
}
