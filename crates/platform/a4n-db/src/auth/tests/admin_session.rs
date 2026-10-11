use crate::auth::{
    bootstrap_first_admin, change_password, create_session, create_user, current_user,
    current_user_summary, disable_user, list_sessions, list_users, login_user, login_with_oauth2,
    register_user, rotate_session_refresh_token, update_profile, update_user_role,
    user_authorization_by_id, user_by_id, user_summary_by_id, verify_session_refresh_token,
};
use a4n_error::AppError;

use super::support::{
    deactivate_user, find_user_by_email, find_user_by_username, session_count, set_user_active,
    setup, user_count,
};

#[tokio::test]
async fn create_user_stores_requested_role() {
    let db = setup().await;

    let admin = create_user(db.as_ref(), "root", "root@example.com", "pw", "admin")
        .await
        .unwrap();
    let regular = create_user(db.as_ref(), "member", "member@example.com", "pw", "user")
        .await
        .unwrap();

    assert_eq!(admin.role, "admin");
    assert_eq!(regular.role, "user");
}

#[tokio::test]
async fn create_user_rejects_unknown_role_before_insert() {
    let db = setup().await;

    let err = create_user(db.as_ref(), "bad", "bad@example.com", "pw", "guest")
        .await
        .expect_err("unknown role must be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert_eq!(user_count(&db).await, 0);
}

#[tokio::test]
async fn list_users_returns_ordered_summaries() {
    let db = setup().await;
    let first = create_user(db.as_ref(), "first", "first@example.com", "pw", "admin")
        .await
        .unwrap();
    let second = create_user(db.as_ref(), "second", "second@example.com", "pw", "user")
        .await
        .unwrap();

    let users = list_users(db.as_ref()).await.unwrap();

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].id, first.id);
    assert_eq!(users[0].username, "first");
    assert_eq!(users[0].role, "admin");
    assert_eq!(users[1].id, second.id);
    assert_eq!(users[1].username, "second");
    assert_eq!(users[1].role, "user");
    assert!(users.iter().all(|user| user.is_active));
    let created_at =
        chrono::DateTime::parse_from_rfc3339(users[0].created_at.as_deref().expect("created_at"))
            .expect("public timestamps must identify their timezone");
    assert_eq!(created_at.naive_utc(), first.created_at);
}

#[tokio::test]
async fn disabled_user_has_admin_summary_but_no_active_session_summary() {
    let db = setup().await;
    let user = create_user(
        db.as_ref(),
        "disabled",
        "disabled@example.com",
        "pw",
        "user",
    )
    .await
    .unwrap();
    deactivate_user(&db, user.id).await;

    let authorization = user_authorization_by_id(db.as_ref(), user.id)
        .await
        .unwrap();
    assert_eq!(authorization.id, user.id);
    assert_eq!(authorization.role, "user");
    assert!(!authorization.is_active);
    assert!(
        !user_summary_by_id(db.as_ref(), user.id)
            .await
            .unwrap()
            .is_active
    );
    assert!(matches!(
        current_user_summary(db.as_ref(), user.id).await,
        Err(AppError::Unauthorized(_))
    ));
}

#[tokio::test]
async fn update_user_role_rejects_unknown_role_without_update() {
    let db = setup().await;
    let user = create_user(db.as_ref(), "alice", "alice@example.com", "pw", "user")
        .await
        .unwrap();

    let err = update_user_role(db.as_ref(), user.id, "guest")
        .await
        .expect_err("unknown role must be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert_eq!(user_by_id(db.as_ref(), user.id).await.unwrap().role, "user");
}

#[tokio::test]
async fn administrator_removal_preserves_one_available_admin_under_concurrency() {
    let db = setup().await;
    let first = create_user(
        db.as_ref(),
        "first-admin",
        "first-admin@example.com",
        "pw",
        "admin",
    )
    .await
    .unwrap();
    let error = update_user_role(db.as_ref(), first.id, "user")
        .await
        .unwrap_err();
    assert!(error.to_string().contains("至少一位可用的管理员"));
    let second = create_user(
        db.as_ref(),
        "second-admin",
        "second-admin@example.com",
        "pw",
        "admin",
    )
    .await
    .unwrap();
    let (a, b) = tokio::join!(
        update_user_role(db.as_ref(), first.id, "user"),
        update_user_role(db.as_ref(), second.id, "user"),
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let remaining = list_users(db.as_ref())
        .await
        .unwrap()
        .into_iter()
        .find(|user| user.role == "admin")
        .unwrap();
    let other_id = if remaining.id == first.id {
        second.id
    } else {
        first.id
    };
    let error = crate::generation::begin_user_disable_gate(db.as_ref(), other_id, remaining.id)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("至少一位可用的管理员"));
    assert!(
        user_by_id(db.as_ref(), remaining.id)
            .await
            .unwrap()
            .is_active
    );
}

#[tokio::test]
async fn disable_user_rejects_self_disable_without_update() {
    let db = setup().await;
    let user = create_user(db.as_ref(), "alice", "alice@example.com", "pw", "admin")
        .await
        .unwrap();

    let err = disable_user(db.as_ref(), user.id, user.id)
        .await
        .expect_err("user must not disable itself");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(user_by_id(db.as_ref(), user.id).await.unwrap().is_active);
}

#[tokio::test]
async fn bootstrap_first_admin_only_creates_on_empty_user_table() {
    let db = setup().await;

    let created = bootstrap_first_admin(db.as_ref(), "root", "root@example.com", "pw")
        .await
        .unwrap()
        .expect("empty database should create first admin");
    let skipped = bootstrap_first_admin(db.as_ref(), "other", "other@example.com", "pw")
        .await
        .unwrap();

    assert_eq!(created.role, "admin");
    assert!(skipped.is_none());
    assert_eq!(user_count(&db).await, 1);
    assert!(find_user_by_username(&db, "other").await.is_none());
}

#[tokio::test]
async fn change_password_rejects_invalid_input_without_update_or_session_loss() {
    let db = setup().await;
    let user = register_user(db.as_ref(), "alice", "alice@example.com", "old-password")
        .await
        .unwrap();

    let session = create_session(db.as_ref(), user.id, "refresh-password-edit", None)
        .await
        .unwrap();

    for (current, next) in [("old-password", ""), ("wrong-password", "new-password")] {
        let err = change_password(db.as_ref(), user.id, current, next)
            .await
            .expect_err("invalid password input must not invalidate the authenticated session");
        assert!(matches!(err, AppError::InvalidArgument(_)));
    }

    verify_session_refresh_token(db.as_ref(), user.id, &session.id, "refresh-password-edit")
        .await
        .unwrap();

    let logged_in = login_user(db.as_ref(), "alice", "old-password")
        .await
        .unwrap();
    assert_eq!(logged_in.id, user.id);
    let blank_login = login_user(db.as_ref(), "alice", "").await;
    assert!(matches!(blank_login, Err(AppError::Unauthorized(_))));
}

#[tokio::test]
async fn verify_session_refresh_token_rejects_disabled_user() {
    let db = setup().await;
    let user = register_user(db.as_ref(), "alice", "alice@example.com", "old-password")
        .await
        .unwrap();
    let refresh_token = "refresh-disabled-user";
    let session = create_session(db.as_ref(), user.id, refresh_token, Some("test-agent"))
        .await
        .unwrap();

    deactivate_user(&db, user.id).await;

    let err = verify_session_refresh_token(db.as_ref(), user.id, &session.id, refresh_token)
        .await
        .expect_err("disabled user must not refresh session");

    assert!(matches!(err, AppError::Unauthorized(_)));
}

#[tokio::test]
async fn refresh_rotation_rejects_stale_old_refresh_after_concurrent_rotation() {
    let db = setup().await;
    let user = register_user(db.as_ref(), "alice", "alice@example.com", "old-password")
        .await
        .unwrap();
    let old_refresh_token = "refresh-old";
    let session = create_session(db.as_ref(), user.id, old_refresh_token, Some("test-agent"))
        .await
        .unwrap();
    let stale_session =
        verify_session_refresh_token(db.as_ref(), user.id, &session.id, old_refresh_token)
            .await
            .unwrap();
    let first_refresh_token = "refresh-first";
    let second_refresh_token = "refresh-second";

    rotate_session_refresh_token(
        db.as_ref(),
        stale_session.clone(),
        old_refresh_token,
        first_refresh_token,
    )
    .await
    .unwrap();

    let err = rotate_session_refresh_token(
        db.as_ref(),
        stale_session,
        old_refresh_token,
        second_refresh_token,
    )
    .await
    .expect_err("stale refresh rotation must not overwrite the first rotation");

    assert!(matches!(err, AppError::Unauthorized(_)));
    let after =
        verify_session_refresh_token(db.as_ref(), user.id, &session.id, first_refresh_token)
            .await
            .expect("first returned refresh token must remain valid");
    assert_eq!(after.id, session.id);
    let old_result =
        verify_session_refresh_token(db.as_ref(), user.id, &session.id, old_refresh_token).await;
    assert!(matches!(old_result, Err(AppError::Unauthorized(_))));
}

#[tokio::test]
async fn get_current_user_rejects_disabled_user_without_breaking_raw_get() {
    let db = setup().await;
    let user = register_user(db.as_ref(), "alice", "alice@example.com", "old-password")
        .await
        .unwrap();

    let summary = current_user_summary(db.as_ref(), user.id)
        .await
        .expect("active user summary must be readable");
    assert_eq!(summary.id, user.id);
    assert_eq!(summary.username, "alice");
    assert_eq!(summary.email, "alice@example.com");
    assert!(summary.is_active);

    deactivate_user(&db, user.id).await;

    let raw = user_by_id(db.as_ref(), user.id).await.unwrap();
    assert!(!raw.is_active, "raw get remains available for admin reads");

    let err = current_user(db.as_ref(), user.id)
        .await
        .expect_err("disabled user must not be restored as current user");

    assert!(matches!(err, AppError::Unauthorized(_)));

    let summary_err = current_user_summary(db.as_ref(), user.id)
        .await
        .expect_err("disabled user summary must be rejected");
    assert!(matches!(summary_err, AppError::Unauthorized(_)));
    assert!(matches!(
        login_user(db.as_ref(), "alice", "old-password").await,
        Err(AppError::Unauthorized(_))
    ));
    assert!(matches!(
        login_with_oauth2(
            db.as_ref(),
            "google",
            "disabled-subject",
            Some("alice@example.com"),
            Some("alice"),
            None,
        )
        .await,
        Err(AppError::Unauthorized(_))
    ));
    let after = user_by_id(db.as_ref(), user.id).await.unwrap();
    assert_eq!(after.oauth_provider, None);
    assert_eq!(after.last_login_at, raw.last_login_at);
}

#[tokio::test]
async fn update_profile_rejects_disabled_user_without_update() {
    let db = setup().await;
    let user = register_user(db.as_ref(), "alice", "alice@example.com", "old-password")
        .await
        .unwrap();

    deactivate_user(&db, user.id).await;

    let err = update_profile(
        db.as_ref(),
        user.id,
        Some("new@example.com"),
        Some(Some("avatar.png")),
        None,
    )
    .await
    .expect_err("disabled user must not update profile");

    assert!(matches!(err, AppError::Unauthorized(_)));
    let after = user_by_id(db.as_ref(), user.id).await.unwrap();
    assert_eq!(after.email, "alice@example.com");
    assert_eq!(after.avatar_url, None);
    assert!(find_user_by_email(&db, "new@example.com").await.is_none());
}

#[tokio::test]
async fn change_password_rejects_disabled_user_without_update() {
    let db = setup().await;
    let user = register_user(db.as_ref(), "alice", "alice@example.com", "old-password")
        .await
        .unwrap();

    deactivate_user(&db, user.id).await;

    let err = change_password(db.as_ref(), user.id, "old-password", "new-password")
        .await
        .expect_err("disabled user must not change password");

    assert!(matches!(err, AppError::Unauthorized(_)));
    set_user_active(&db, user.id, true).await;
    assert_eq!(
        login_user(db.as_ref(), "alice", "old-password")
            .await
            .unwrap()
            .id,
        user.id
    );
    assert!(matches!(
        login_user(db.as_ref(), "alice", "new-password").await,
        Err(AppError::Unauthorized(_))
    ));
}

#[tokio::test]
async fn create_session_rejects_disabled_user_before_insert() {
    let db = setup().await;
    let user = register_user(db.as_ref(), "alice", "alice@example.com", "old-password")
        .await
        .unwrap();
    let refresh_token = "refresh-disabled-session";

    deactivate_user(&db, user.id).await;

    let err = create_session(db.as_ref(), user.id, refresh_token, Some("test-agent"))
        .await
        .expect_err("disabled user must not create a new session");

    assert!(matches!(err, AppError::Unauthorized(_)));
    assert_eq!(session_count(&db).await, 0);
}

#[tokio::test]
async fn list_sessions_rejects_disabled_user() {
    let db = setup().await;
    let user = register_user(db.as_ref(), "alice", "alice@example.com", "old-password")
        .await
        .unwrap();
    let refresh_token = "refresh-list-sessions";
    create_session(db.as_ref(), user.id, refresh_token, Some("test-agent"))
        .await
        .unwrap();

    deactivate_user(&db, user.id).await;

    let err = list_sessions(db.as_ref(), user.id)
        .await
        .expect_err("disabled user must not list sessions");

    assert!(matches!(err, AppError::Unauthorized(_)));
    assert_eq!(session_count(&db).await, 1);
}
