use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::models::user::{invite_code, user, user_session};
use sea_orm::{ActiveModelTrait, EntityTrait, PaginatorTrait, Set};

use super::{AuthService, jwt};

async fn setup() -> AuthService {
    let db = a4n_migration::test_support::connect_test_database().await;
    AuthService::new(Arc::new(db))
}

async fn deactivate_user(svc: &AuthService, user_id: i64) {
    set_user_active(svc, user_id, false).await;
}

async fn set_user_active(svc: &AuthService, user_id: i64, is_active: bool) {
    let user = svc.get(user_id).await.unwrap();
    let mut active: user::ActiveModel = user.into();
    active.is_active = Set(is_active);
    active.update(svc.db.as_ref()).await.unwrap();
}

async fn user_count(svc: &AuthService) -> u64 {
    user::Entity::find().count(svc.db.as_ref()).await.unwrap()
}

async fn session_count(svc: &AuthService) -> u64 {
    user_session::Entity::find()
        .count(svc.db.as_ref())
        .await
        .unwrap()
}

async fn invite_code_count(svc: &AuthService) -> u64 {
    invite_code::Entity::find()
        .count(svc.db.as_ref())
        .await
        .unwrap()
}

#[tokio::test]
async fn register_trims_username_before_uniqueness_check() {
    let svc = setup().await;
    let user = svc
        .register(" alice ", "alice@example.com", "pw")
        .await
        .unwrap();

    assert_eq!(user.username, "alice");

    let err = svc
        .register("alice", "alice2@example.com", "pw")
        .await
        .expect_err("trimmed username identity must be unique");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[tokio::test]
async fn register_with_invite_consumes_code_once() {
    let svc = setup().await;
    svc.create_invite_code("INV-1", None).await.unwrap();

    let user = svc
        .register_with_invite("alice", "alice@example.com", "pw", "INV-1")
        .await
        .unwrap();
    let codes = svc.list_invite_codes().await.unwrap();
    let code = codes
        .iter()
        .find(|item| item.code == "INV-1")
        .expect("invite code must remain listed");

    assert_eq!(code.used_by_user_id, Some(user.id));
    assert!(code.used_at.is_some());

    let err = svc
        .register_with_invite("bob", "bob@example.com", "pw", "INV-1")
        .await
        .expect_err("used invite code must be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert_eq!(user_count(&svc).await, 1);
}

#[tokio::test]
async fn register_with_invite_rejects_blank_code_without_insert() {
    let svc = setup().await;

    let err = svc
        .register_with_invite("alice", "alice@example.com", "pw", "   ")
        .await
        .expect_err("blank invite code must be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert_eq!(user_count(&svc).await, 0);
    assert_eq!(invite_code_count(&svc).await, 0);
}

#[tokio::test]
async fn register_with_invite_rejects_unknown_code_without_insert() {
    let svc = setup().await;

    let err = svc
        .register_with_invite("alice", "alice@example.com", "pw", "MISSING")
        .await
        .expect_err("unknown invite code must be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert_eq!(user_count(&svc).await, 0);
}

#[tokio::test]
async fn create_invite_code_rejects_duplicate_code_after_trim() {
    let svc = setup().await;
    svc.create_invite_code(" INV-1 ", None).await.unwrap();

    let err = svc
        .create_invite_code("INV-1", None)
        .await
        .expect_err("duplicate invite code must be rejected before insert");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert_eq!(invite_code_count(&svc).await, 1);
}

#[tokio::test]
async fn login_uses_trimmed_username_identity() {
    let svc = setup().await;
    let user = svc
        .register("alice", "alice@example.com", "pw")
        .await
        .unwrap();

    let logged_in = svc.login(" alice ", "pw").await.unwrap();

    assert_eq!(logged_in.id, user.id);
    assert_eq!(logged_in.username, "alice");
}

#[tokio::test]
async fn register_rejects_blank_username_before_insert() {
    let svc = setup().await;

    let err = svc
        .register("   ", "blank@example.com", "pw")
        .await
        .expect_err("blank username must be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(
        svc.find_by_email("blank@example.com")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn register_rejects_blank_password_before_insert() {
    let svc = setup().await;

    let err = svc
        .register("alice", "alice@example.com", "   ")
        .await
        .expect_err("blank password must be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(svc.find_by_username("alice").await.unwrap().is_none());
}

#[tokio::test]
async fn register_stores_salted_argon2id_password_hash() {
    let svc = setup().await;

    let user = svc
        .register("alice", "alice@example.com", "pw")
        .await
        .unwrap();
    let other = svc.register("bob", "bob@example.com", "pw").await.unwrap();

    assert!(user.password_hash.starts_with("$argon2"));
    assert_ne!(user.password_hash, "pw");
    assert_ne!(user.password_hash, other.password_hash);
    assert_eq!(svc.login("alice", "pw").await.unwrap().id, user.id);
}

#[tokio::test]
async fn login_reports_invalid_password_hash_format() {
    let svc = setup().await;
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
    .insert(svc.db.as_ref())
    .await
    .unwrap();

    let err = svc
        .login("bad_hash", "pw")
        .await
        .expect_err("invalid password hash format must be reported");

    assert!(matches!(err, AppError::Internal(_)));
}

#[tokio::test]
async fn create_user_stores_requested_role() {
    let svc = setup().await;

    let admin = svc
        .create_user("root", "root@example.com", "pw", "admin")
        .await
        .unwrap();
    let read_only = svc
        .create_user("reader", "reader@example.com", "pw", "read_only")
        .await
        .unwrap();

    assert_eq!(admin.role, "admin");
    assert_eq!(read_only.role, "read_only");
}

#[tokio::test]
async fn create_user_rejects_unknown_role_before_insert() {
    let svc = setup().await;

    let err = svc
        .create_user("bad", "bad@example.com", "pw", "guest")
        .await
        .expect_err("unknown role must be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert_eq!(user_count(&svc).await, 0);
}

#[tokio::test]
async fn update_user_role_rejects_unknown_role_without_update() {
    let svc = setup().await;
    let user = svc
        .create_user("alice", "alice@example.com", "pw", "user")
        .await
        .unwrap();

    let err = svc
        .update_user_role(user.id, "guest")
        .await
        .expect_err("unknown role must be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert_eq!(svc.get(user.id).await.unwrap().role, "user");
}

#[tokio::test]
async fn disable_user_rejects_self_disable_without_update() {
    let svc = setup().await;
    let user = svc
        .create_user("alice", "alice@example.com", "pw", "admin")
        .await
        .unwrap();

    let err = svc
        .disable_user(user.id, user.id)
        .await
        .expect_err("user must not disable itself");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(svc.get(user.id).await.unwrap().is_active);
}

#[tokio::test]
async fn bootstrap_first_admin_only_creates_on_empty_user_table() {
    let svc = setup().await;

    let created = svc
        .bootstrap_first_admin("root", "root@example.com", "pw")
        .await
        .unwrap()
        .expect("empty database should create first admin");
    let skipped = svc
        .bootstrap_first_admin("other", "other@example.com", "pw")
        .await
        .unwrap();

    assert_eq!(created.role, "admin");
    assert!(skipped.is_none());
    assert_eq!(user_count(&svc).await, 1);
    assert!(svc.find_by_username("other").await.unwrap().is_none());
}

#[tokio::test]
async fn change_password_rejects_blank_new_password_without_update() {
    let svc = setup().await;
    let user = svc
        .register("alice", "alice@example.com", "old-password")
        .await
        .unwrap();

    let err = svc
        .change_password(user.id, "old-password", "")
        .await
        .expect_err("blank new password must be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    let logged_in = svc.login("alice", "old-password").await.unwrap();
    assert_eq!(logged_in.id, user.id);
    let blank_login = svc.login("alice", "").await;
    assert!(matches!(blank_login, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn verify_session_refresh_token_rejects_disabled_user() {
    let svc = setup().await;
    let user = svc
        .register("alice", "alice@example.com", "old-password")
        .await
        .unwrap();
    let refresh_token = jwt::create_refresh_token(&user.id.to_string(), "test-secret", 7).unwrap();
    let session = svc
        .create_session(user.id, &refresh_token, Some("test-agent"))
        .await
        .unwrap();

    deactivate_user(&svc, user.id).await;

    let err = svc
        .verify_session_refresh_token(user.id, &session.id, &refresh_token)
        .await
        .expect_err("disabled user must not refresh session");

    assert!(matches!(err, AppError::Forbidden(_)));
}

#[tokio::test]
async fn refresh_rotation_rejects_stale_old_refresh_after_concurrent_rotation() {
    let svc = setup().await;
    let user = svc
        .register("alice", "alice@example.com", "old-password")
        .await
        .unwrap();
    let old_refresh_token = jwt::create_refresh_token(&user.id.to_string(), "test-secret", 7)
        .expect("old refresh token");
    let session = svc
        .create_session(user.id, &old_refresh_token, Some("test-agent"))
        .await
        .unwrap();
    let stale_session = svc
        .verify_session_refresh_token(user.id, &session.id, &old_refresh_token)
        .await
        .unwrap();
    let first_refresh_token = jwt::create_refresh_token(&user.id.to_string(), "test-secret", 7)
        .expect("first refresh token");
    let second_refresh_token = jwt::create_refresh_token(&user.id.to_string(), "test-secret", 7)
        .expect("second refresh token");

    svc.rotate_session_refresh_token(
        stale_session.clone(),
        &old_refresh_token,
        &first_refresh_token,
    )
    .await
    .unwrap();

    let err = svc
        .rotate_session_refresh_token(stale_session, &old_refresh_token, &second_refresh_token)
        .await
        .expect_err("stale refresh rotation must not overwrite the first rotation");

    assert!(matches!(err, AppError::Forbidden(_)));
    let after = svc
        .verify_session_refresh_token(user.id, &session.id, &first_refresh_token)
        .await
        .expect("first returned refresh token must remain valid");
    assert_eq!(after.id, session.id);
    let old_result = svc
        .verify_session_refresh_token(user.id, &session.id, &old_refresh_token)
        .await;
    assert!(matches!(old_result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn get_current_user_rejects_disabled_user_without_breaking_raw_get() {
    let svc = setup().await;
    let user = svc
        .register("alice", "alice@example.com", "old-password")
        .await
        .unwrap();

    deactivate_user(&svc, user.id).await;

    let raw = svc.get(user.id).await.unwrap();
    assert!(!raw.is_active, "raw get remains available for admin reads");

    let err = svc
        .get_current_user(user.id)
        .await
        .expect_err("disabled user must not be restored as current user");

    assert!(matches!(err, AppError::Forbidden(_)));
}

#[tokio::test]
async fn update_profile_rejects_disabled_user_without_update() {
    let svc = setup().await;
    let user = svc
        .register("alice", "alice@example.com", "old-password")
        .await
        .unwrap();

    deactivate_user(&svc, user.id).await;

    let err = svc
        .update_profile(user.id, Some("new@example.com"), Some(Some("avatar.png")))
        .await
        .expect_err("disabled user must not update profile");

    assert!(matches!(err, AppError::Forbidden(_)));
    let after = svc.get(user.id).await.unwrap();
    assert_eq!(after.email, "alice@example.com");
    assert_eq!(after.avatar_url, None);
    assert!(
        svc.find_by_email("new@example.com")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn change_password_rejects_disabled_user_without_update() {
    let svc = setup().await;
    let user = svc
        .register("alice", "alice@example.com", "old-password")
        .await
        .unwrap();

    deactivate_user(&svc, user.id).await;

    let err = svc
        .change_password(user.id, "old-password", "new-password")
        .await
        .expect_err("disabled user must not change password");

    assert!(matches!(err, AppError::Forbidden(_)));
    set_user_active(&svc, user.id, true).await;
    assert_eq!(
        svc.login("alice", "old-password").await.unwrap().id,
        user.id
    );
    assert!(matches!(
        svc.login("alice", "new-password").await,
        Err(AppError::Forbidden(_))
    ));
}

#[tokio::test]
async fn create_session_rejects_disabled_user_before_insert() {
    let svc = setup().await;
    let user = svc
        .register("alice", "alice@example.com", "old-password")
        .await
        .unwrap();
    let refresh_token = jwt::create_refresh_token(&user.id.to_string(), "test-secret", 7).unwrap();

    deactivate_user(&svc, user.id).await;

    let err = svc
        .create_session(user.id, &refresh_token, Some("test-agent"))
        .await
        .expect_err("disabled user must not create a new session");

    assert!(matches!(err, AppError::Forbidden(_)));
    assert_eq!(session_count(&svc).await, 0);
}

#[tokio::test]
async fn list_sessions_rejects_disabled_user() {
    let svc = setup().await;
    let user = svc
        .register("alice", "alice@example.com", "old-password")
        .await
        .unwrap();
    let refresh_token = jwt::create_refresh_token(&user.id.to_string(), "test-secret", 7).unwrap();
    svc.create_session(user.id, &refresh_token, Some("test-agent"))
        .await
        .unwrap();

    deactivate_user(&svc, user.id).await;

    let err = svc
        .list_sessions(user.id)
        .await
        .expect_err("disabled user must not list sessions");

    assert!(matches!(err, AppError::Forbidden(_)));
    assert_eq!(session_count(&svc).await, 1);
}

#[tokio::test]
async fn register_trims_email_before_uniqueness_check() {
    let svc = setup().await;
    let user = svc
        .register("alice", " alice@example.com ", "pw")
        .await
        .unwrap();

    assert_eq!(user.email, "alice@example.com");

    let err = svc
        .register("alice2", "alice@example.com", "pw")
        .await
        .expect_err("trimmed email identity must be unique");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[tokio::test]
async fn register_rejects_email_with_internal_whitespace_before_insert() {
    let svc = setup().await;

    let err = svc
        .register("alice", "alice @example.com", "pw")
        .await
        .expect_err("email identity must not contain whitespace");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(svc.find_by_username("alice").await.unwrap().is_none());
    assert_eq!(user_count(&svc).await, 0);
}

#[tokio::test]
async fn update_profile_rejects_email_owned_by_another_user() {
    let svc = setup().await;
    let alice = svc
        .register("alice", "alice@example.com", "pw")
        .await
        .unwrap();
    let bob = svc.register("bob", "bob@example.com", "pw").await.unwrap();

    let err = svc
        .update_profile(bob.id, Some(" alice@example.com "), None)
        .await
        .expect_err("email identity must not move to another user");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    let bob_after = svc.get(bob.id).await.unwrap();
    assert_eq!(bob_after.email, "bob@example.com");
    let alice_after = svc.get(alice.id).await.unwrap();
    assert_eq!(alice_after.email, "alice@example.com");
}

#[tokio::test]
async fn update_profile_rejects_email_with_internal_whitespace_without_update() {
    let svc = setup().await;
    let user = svc
        .register("alice", "alice@example.com", "pw")
        .await
        .unwrap();

    let err = svc
        .update_profile(user.id, Some("alice @example.com"), None)
        .await
        .expect_err("profile email identity must not contain whitespace");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    let after = svc.get(user.id).await.unwrap();
    assert_eq!(after.email, "alice@example.com");
    assert!(
        svc.find_by_email("alice@example.com")
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn update_profile_clears_avatar_when_explicitly_null() {
    let svc = setup().await;
    let user = svc
        .register("alice", "alice@example.com", "pw")
        .await
        .unwrap();
    let with_avatar = svc
        .update_profile(user.id, None, Some(Some(" https://cdn/avatar.png ")))
        .await
        .unwrap();

    assert_eq!(
        with_avatar.avatar_url.as_deref(),
        Some("https://cdn/avatar.png")
    );

    let cleared = svc.update_profile(user.id, None, Some(None)).await.unwrap();

    assert_eq!(cleared.avatar_url, None);
}

#[tokio::test]
async fn update_profile_keeps_avatar_when_avatar_field_is_missing() {
    let svc = setup().await;
    let user = svc
        .register("alice", "alice@example.com", "pw")
        .await
        .unwrap();
    svc.update_profile(user.id, None, Some(Some("https://cdn/avatar.png")))
        .await
        .unwrap();

    let updated = svc
        .update_profile(user.id, Some("alice2@example.com"), None)
        .await
        .unwrap();

    assert_eq!(updated.email, "alice2@example.com");
    assert_eq!(
        updated.avatar_url.as_deref(),
        Some("https://cdn/avatar.png")
    );
}

#[tokio::test]
async fn oauth_first_login_creates_user() {
    let svc = setup().await;
    let u = svc
        .login_with_oauth2("github", "gh-1001", Some("a@b.com"), Some("alice"), None)
        .await
        .unwrap();
    assert_eq!(u.username, "alice");
    assert_eq!(u.oauth_provider.as_deref(), Some("github"));
    assert_eq!(u.oauth_subject.as_deref(), Some("gh-1001"));
    assert!(u.last_login_at.is_some());
}

#[tokio::test]
async fn oauth_second_login_returns_same_user() {
    let svc = setup().await;
    let u1 = svc
        .login_with_oauth2("github", "gh-2001", Some("b@b.com"), Some("bob"), None)
        .await
        .unwrap();
    let u2 = svc
        .login_with_oauth2("github", "gh-2001", Some("b@b.com"), Some("bob"), None)
        .await
        .unwrap();
    assert_eq!(u1.id, u2.id, "同一 provider+subject 必须命中同一账户");
}

#[tokio::test]
async fn oauth_username_conflict_appends_suffix() {
    let svc = setup().await;
    svc.register("dave", "dave@old.com", "pw").await.unwrap();
    let u = svc
        .login_with_oauth2(
            "github",
            "gh-3001",
            Some("dave@new.com"),
            Some("dave"),
            None,
        )
        .await
        .unwrap();
    // 由于 dave 邮箱不同，按 email 关联失败，会走自动注册分支并追加后缀
    assert_ne!(u.username, "dave");
    assert!(u.username.starts_with("dave"));
    assert_eq!(u.oauth_provider.as_deref(), Some("github"));
}

#[tokio::test]
async fn oauth_links_to_existing_email() {
    let svc = setup().await;
    let existing = svc.register("eve", "eve@old.com", "pw").await.unwrap();
    let linked = svc
        .login_with_oauth2("google", "g-4001", Some("eve@old.com"), Some("eve2"), None)
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
    let svc = setup().await;

    let err = svc
        .login_with_oauth2(
            "   ",
            "gh-blank-provider",
            Some("bad@example.com"),
            Some("bad"),
            None,
        )
        .await
        .expect_err("blank OAuth provider must be rejected before insertion");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(
        svc.find_by_email("bad@example.com")
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(user_count(&svc).await, 0);
}

#[tokio::test]
async fn oauth_rejects_blank_provider_subject_before_insert() {
    let svc = setup().await;

    let err = svc
        .login_with_oauth2("github", "   ", Some("bad@example.com"), Some("bad"), None)
        .await
        .expect_err("blank OAuth provider subject must be rejected before insertion");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(
        svc.find_by_email("bad@example.com")
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(user_count(&svc).await, 0);
}

#[tokio::test]
async fn oauth_trims_provider_identity_before_lookup_and_storage() {
    let svc = setup().await;
    let first = svc
        .login_with_oauth2(
            "github",
            "gh-trim-1",
            Some("alice@example.com"),
            Some("alice"),
            None,
        )
        .await
        .unwrap();

    let second = svc
        .login_with_oauth2(
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
    assert!(
        svc.find_by_email("other@example.com")
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(user_count(&svc).await, 1);
}

#[tokio::test]
async fn oauth_rejects_unsupported_provider_before_insert() {
    let svc = setup().await;

    let err = svc
        .login_with_oauth2(
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
        svc.find_by_email("gitlab@example.com")
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(user_count(&svc).await, 0);
}

#[tokio::test]
async fn oauth_new_user_requires_email() {
    let svc = setup().await;
    let err = svc
        .login_with_oauth2("github", "gh-5001", None, Some("frank"), None)
        .await
        .expect_err("OAuth auto registration must not fabricate email");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[tokio::test]
async fn oauth_new_user_rejects_invalid_email() {
    let svc = setup().await;
    let err = svc
        .login_with_oauth2(
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
    let svc = setup().await;

    let err = svc
        .login_with_oauth2(
            "github",
            "gh-space-email",
            Some("oauth @example.com"),
            Some("oauth"),
            None,
        )
        .await
        .expect_err("OAuth email identity must not contain whitespace");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(svc.find_by_username("oauth").await.unwrap().is_none());
    assert_eq!(user_count(&svc).await, 0);
}

#[tokio::test]
async fn oauth_new_user_uses_email_local_part_when_username_missing() {
    let svc = setup().await;
    let u = svc
        .login_with_oauth2("github", "gh-5003", Some("grace@example.com"), None, None)
        .await
        .unwrap();

    assert_eq!(u.username, "grace");
    assert_eq!(u.email, "grace@example.com");
    assert!(!u.email.ends_with(".oauth.local"));
}
