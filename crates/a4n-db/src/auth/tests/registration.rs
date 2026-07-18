use crate::auth::{create_invite_code, login_user, register_user, register_user_with_invite};
use a4n_core::{
    error::AppError,
    models::user::{invite_code, user},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

use super::support::{
    find_user_by_email, find_user_by_username, invite_code_count, setup, user_count,
};

#[tokio::test]
async fn register_trims_username_before_uniqueness_check() {
    let db = setup().await;
    let user = register_user(db.as_ref(), " alice ", "alice@example.com", "pw")
        .await
        .unwrap();

    assert_eq!(user.username, "alice");

    let err = register_user(db.as_ref(), "alice", "alice2@example.com", "pw")
        .await
        .expect_err("trimmed username identity must be unique");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[tokio::test]
async fn register_with_invite_consumes_code_once() {
    let db = setup().await;
    create_invite_code(db.as_ref(), "INV-1", None)
        .await
        .unwrap();

    let user = register_user_with_invite(db.as_ref(), "alice", "alice@example.com", "pw", "INV-1")
        .await
        .unwrap();
    let code = invite_code::Entity::find()
        .filter(invite_code::Column::Code.eq("INV-1"))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("invite code must remain stored");

    assert_eq!(code.used_by_user_id, Some(user.id));
    assert!(code.used_at.is_some());

    let err = register_user_with_invite(db.as_ref(), "bob", "bob@example.com", "pw", "INV-1")
        .await
        .expect_err("used invite code must be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert_eq!(user_count(&db).await, 1);
}

#[tokio::test]
async fn register_with_invite_rejects_blank_code_without_insert() {
    let db = setup().await;

    let err = register_user_with_invite(db.as_ref(), "alice", "alice@example.com", "pw", "   ")
        .await
        .expect_err("blank invite code must be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert_eq!(user_count(&db).await, 0);
    assert_eq!(invite_code_count(&db).await, 0);
}

#[tokio::test]
async fn register_with_invite_rejects_unknown_code_without_insert() {
    let db = setup().await;

    let err = register_user_with_invite(db.as_ref(), "alice", "alice@example.com", "pw", "MISSING")
        .await
        .expect_err("unknown invite code must be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert_eq!(user_count(&db).await, 0);
}

#[tokio::test]
async fn create_invite_code_rejects_duplicate_code_after_trim() {
    let db = setup().await;
    create_invite_code(db.as_ref(), " INV-1 ", None)
        .await
        .unwrap();

    let err = create_invite_code(db.as_ref(), "INV-1", None)
        .await
        .expect_err("duplicate invite code must be rejected before insert");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert_eq!(invite_code_count(&db).await, 1);
}

#[tokio::test]
async fn login_uses_trimmed_username_identity() {
    let db = setup().await;
    let user = register_user(db.as_ref(), "alice", "alice@example.com", "pw")
        .await
        .unwrap();

    let logged_in = login_user(db.as_ref(), " alice ", "pw").await.unwrap();

    assert_eq!(logged_in.id, user.id);
    assert_eq!(logged_in.username, "alice");
}

#[tokio::test]
async fn register_rejects_blank_username_before_insert() {
    let db = setup().await;

    let err = register_user(db.as_ref(), "   ", "blank@example.com", "pw")
        .await
        .expect_err("blank username must be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(find_user_by_email(&db, "blank@example.com").await.is_none());
}

#[tokio::test]
async fn register_rejects_blank_password_before_insert() {
    let db = setup().await;

    let err = register_user(db.as_ref(), "alice", "alice@example.com", "   ")
        .await
        .expect_err("blank password must be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(find_user_by_username(&db, "alice").await.is_none());
}

#[tokio::test]
async fn register_stores_salted_argon2id_password_hash() {
    let db = setup().await;

    let user = register_user(db.as_ref(), "alice", "alice@example.com", "pw")
        .await
        .unwrap();
    let other = register_user(db.as_ref(), "bob", "bob@example.com", "pw")
        .await
        .unwrap();

    assert!(user.password_hash.starts_with("$argon2"));
    assert_ne!(user.password_hash, "pw");
    assert_ne!(user.password_hash, other.password_hash);
    assert_eq!(
        login_user(db.as_ref(), "alice", "pw").await.unwrap().id,
        user.id
    );
}

#[tokio::test]
async fn login_reports_invalid_password_hash_format() {
    let db = setup().await;
    let now = chrono::Utc::now().naive_utc();
    user::ActiveModel {
        username: Set("bad_hash".to_string()),
        email: Set("bad_hash@example.com".to_string()),
        password_hash: Set("not-an-argon2-hash".to_string()),
        role: Set("user".to_string()),
        oauth_provider: Set(None),
        oauth_subject: Set(None),
        avatar_url: Set(None),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        last_login_at: Set(None),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();

    let err = login_user(db.as_ref(), "bad_hash", "pw")
        .await
        .expect_err("invalid password hash format must be reported");

    assert!(matches!(err, AppError::Internal(_)));
}
