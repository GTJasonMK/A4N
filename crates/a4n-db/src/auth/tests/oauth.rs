use crate::auth::{login_with_oauth2, register_user};
use a4n_core::error::AppError;

use super::support::{find_user_by_email, find_user_by_username, setup, user_count};

#[tokio::test]
async fn oauth_first_login_creates_user() {
    let db = setup().await;
    let u = login_with_oauth2(
        db.as_ref(),
        "github",
        "gh-1001",
        Some("a@b.com"),
        Some("alice"),
        None,
    )
    .await
    .unwrap();
    assert_eq!(u.username, "alice");
    assert_eq!(u.oauth_provider.as_deref(), Some("github"));
    assert_eq!(u.oauth_subject.as_deref(), Some("gh-1001"));
    assert!(u.last_login_at.is_some());
}

#[tokio::test]
async fn oauth_second_login_returns_same_user() {
    let db = setup().await;
    let u1 = login_with_oauth2(
        db.as_ref(),
        "github",
        "gh-2001",
        Some("b@b.com"),
        Some("bob"),
        None,
    )
    .await
    .unwrap();
    let u2 = login_with_oauth2(
        db.as_ref(),
        "github",
        "gh-2001",
        Some("b@b.com"),
        Some("bob"),
        None,
    )
    .await
    .unwrap();
    assert_eq!(u1.id, u2.id, "同一 provider+subject 必须命中同一账户");
}

#[tokio::test]
async fn oauth_username_conflict_appends_suffix() {
    let db = setup().await;
    register_user(db.as_ref(), "dave", "dave@old.com", "pw")
        .await
        .unwrap();
    for n in 2..=8 {
        register_user(
            db.as_ref(),
            &format!("dave_{n}"),
            &format!("dave_{n}@old.com"),
            "pw",
        )
        .await
        .unwrap();
    }
    let u = login_with_oauth2(
        db.as_ref(),
        "github",
        "gh-3001",
        Some("dave@new.com"),
        Some("dave"),
        None,
    )
    .await
    .unwrap();
    // 由于 dave 邮箱不同，按 email 关联失败；连续候选占用后应选择批量检查范围的首个空位。
    assert_eq!(u.username, "dave_9");
    assert_eq!(u.oauth_provider.as_deref(), Some("github"));
}

#[tokio::test]
async fn oauth_links_to_existing_email() {
    let db = setup().await;
    let existing = register_user(db.as_ref(), "eve", "eve@old.com", "pw")
        .await
        .unwrap();
    let linked = login_with_oauth2(
        db.as_ref(),
        "google",
        "g-4001",
        Some("eve@old.com"),
        Some("eve2"),
        None,
    )
    .await
    .unwrap();
    assert_eq!(linked.id, existing.id, "邮箱匹配时复用账户");
    assert_eq!(linked.oauth_provider.as_deref(), Some("google"));
    assert_eq!(linked.oauth_subject.as_deref(), Some("g-4001"));
    // 用户名应保持原样
    assert_eq!(linked.username, "eve");
}

#[tokio::test]
async fn oauth_rejects_blank_provider_identity_before_insert() {
    let db = setup().await;

    let err = login_with_oauth2(
        db.as_ref(),
        "   ",
        "gh-blank-provider",
        Some("bad@example.com"),
        Some("bad"),
        None,
    )
    .await
    .expect_err("blank OAuth provider must be rejected before insertion");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(find_user_by_email(&db, "bad@example.com").await.is_none());
    assert_eq!(user_count(&db).await, 0);
}

#[tokio::test]
async fn oauth_rejects_blank_provider_subject_before_insert() {
    let db = setup().await;

    let err = login_with_oauth2(
        db.as_ref(),
        "github",
        "   ",
        Some("bad@example.com"),
        Some("bad"),
        None,
    )
    .await
    .expect_err("blank OAuth provider subject must be rejected before insertion");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(find_user_by_email(&db, "bad@example.com").await.is_none());
    assert_eq!(user_count(&db).await, 0);
}

#[tokio::test]
async fn oauth_trims_provider_identity_before_lookup_and_storage() {
    let db = setup().await;
    let first = login_with_oauth2(
        db.as_ref(),
        "github",
        "gh-trim-1",
        Some("alice@example.com"),
        Some("alice"),
        None,
    )
    .await
    .unwrap();

    let second = login_with_oauth2(
        db.as_ref(),
        " github ",
        " gh-trim-1 ",
        Some("other@example.com"),
        Some("other"),
        None,
    )
    .await
    .unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(second.oauth_provider.as_deref(), Some("github"));
    assert_eq!(second.oauth_subject.as_deref(), Some("gh-trim-1"));
    assert!(find_user_by_email(&db, "other@example.com").await.is_none());
    assert_eq!(user_count(&db).await, 1);
}

#[tokio::test]
async fn oauth_rejects_unsupported_provider_before_insert() {
    let db = setup().await;

    let err = login_with_oauth2(
        db.as_ref(),
        "gitlab",
        "gl-1",
        Some("gitlab@example.com"),
        Some("gitlab"),
        None,
    )
    .await
    .expect_err("unsupported OAuth provider must be rejected before insertion");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(
        find_user_by_email(&db, "gitlab@example.com")
            .await
            .is_none()
    );
    assert_eq!(user_count(&db).await, 0);
}

#[tokio::test]
async fn oauth_new_user_requires_email() {
    let db = setup().await;
    let err = login_with_oauth2(db.as_ref(), "github", "gh-5001", None, Some("frank"), None)
        .await
        .expect_err("OAuth auto registration must not fabricate email");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[tokio::test]
async fn oauth_new_user_rejects_invalid_email() {
    let db = setup().await;
    let err = login_with_oauth2(
        db.as_ref(),
        "github",
        "gh-5002",
        Some("not-an-email"),
        Some("frank"),
        None,
    )
    .await
    .expect_err("OAuth auto registration must require a real email");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[tokio::test]
async fn oauth_rejects_email_with_internal_whitespace_before_insert() {
    let db = setup().await;

    let err = login_with_oauth2(
        db.as_ref(),
        "github",
        "gh-space-email",
        Some("oauth @example.com"),
        Some("oauth"),
        None,
    )
    .await
    .expect_err("OAuth email identity must not contain whitespace");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(find_user_by_username(&db, "oauth").await.is_none());
    assert_eq!(user_count(&db).await, 0);
}

#[tokio::test]
async fn oauth_new_user_uses_email_local_part_when_username_missing() {
    let db = setup().await;
    let u = login_with_oauth2(
        db.as_ref(),
        "github",
        "gh-5003",
        Some("grace@example.com"),
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(u.username, "grace");
    assert_eq!(u.email, "grace@example.com");
    assert!(!u.email.ends_with(".oauth.local"));
}
