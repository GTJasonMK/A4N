use crate::auth::{register_user, update_profile, user_by_id};
use a4n_core::error::AppError;

use super::support::{find_user_by_email, find_user_by_username, setup, user_count};

#[tokio::test]
async fn register_trims_email_before_uniqueness_check() {
    let db = setup().await;
    let user = register_user(db.as_ref(), "alice", " alice@example.com ", "pw")
        .await
        .unwrap();

    assert_eq!(user.email, "alice@example.com");

    let err = register_user(db.as_ref(), "alice2", "alice@example.com", "pw")
        .await
        .expect_err("trimmed email identity must be unique");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[tokio::test]
async fn register_rejects_email_with_internal_whitespace_before_insert() {
    let db = setup().await;

    let err = register_user(db.as_ref(), "alice", "alice @example.com", "pw")
        .await
        .expect_err("email identity must not contain whitespace");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(find_user_by_username(&db, "alice").await.is_none());
    assert_eq!(user_count(&db).await, 0);
}

#[tokio::test]
async fn update_profile_rejects_email_owned_by_another_user() {
    let db = setup().await;
    let alice = register_user(db.as_ref(), "alice", "alice@example.com", "pw")
        .await
        .unwrap();
    let bob = register_user(db.as_ref(), "bob", "bob@example.com", "pw")
        .await
        .unwrap();

    let err = update_profile(db.as_ref(), bob.id, Some(" alice@example.com "), None)
        .await
        .expect_err("email identity must not move to another user");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    let bob_after = user_by_id(db.as_ref(), bob.id).await.unwrap();
    assert_eq!(bob_after.email, "bob@example.com");
    let alice_after = user_by_id(db.as_ref(), alice.id).await.unwrap();
    assert_eq!(alice_after.email, "alice@example.com");
}

#[tokio::test]
async fn update_profile_rejects_email_with_internal_whitespace_without_update() {
    let db = setup().await;
    let user = register_user(db.as_ref(), "alice", "alice@example.com", "pw")
        .await
        .unwrap();

    let err = update_profile(db.as_ref(), user.id, Some("alice @example.com"), None)
        .await
        .expect_err("profile email identity must not contain whitespace");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    let after = user_by_id(db.as_ref(), user.id).await.unwrap();
    assert_eq!(after.email, "alice@example.com");
    assert!(find_user_by_email(&db, "alice@example.com").await.is_some());
}

#[tokio::test]
async fn update_profile_clears_avatar_when_explicitly_null() {
    let db = setup().await;
    let user = register_user(db.as_ref(), "alice", "alice@example.com", "pw")
        .await
        .unwrap();
    let with_avatar = update_profile(
        db.as_ref(),
        user.id,
        None,
        Some(Some(" https://cdn/avatar.png ")),
    )
    .await
    .unwrap();

    assert_eq!(
        with_avatar.avatar_url.as_deref(),
        Some("https://cdn/avatar.png")
    );

    let cleared = update_profile(db.as_ref(), user.id, None, Some(None))
        .await
        .unwrap();

    assert_eq!(cleared.avatar_url, None);
}

#[tokio::test]
async fn update_profile_keeps_avatar_when_avatar_field_is_missing() {
    let db = setup().await;
    let user = register_user(db.as_ref(), "alice", "alice@example.com", "pw")
        .await
        .unwrap();
    update_profile(
        db.as_ref(),
        user.id,
        None,
        Some(Some("https://cdn/avatar.png")),
    )
    .await
    .unwrap();

    let updated = update_profile(db.as_ref(), user.id, Some("alice2@example.com"), None)
        .await
        .unwrap();

    assert_eq!(updated.email, "alice2@example.com");
    assert_eq!(
        updated.avatar_url.as_deref(),
        Some("https://cdn/avatar.png")
    );
}
