use super::contracts::UpdateProfileInput;
use a4n_orchestrator::auth::{
    ensure_same_user, profile_email_update, required_logout_identity, required_session_id,
    required_session_user_id,
};
use serde_json::json;

#[test]
fn update_profile_input_distinguishes_missing_from_null_avatar() {
    let missing: UpdateProfileInput = serde_json::from_value(json!({})).unwrap();
    let null: UpdateProfileInput = serde_json::from_value(json!({ "avatar_url": null })).unwrap();
    let value: UpdateProfileInput =
        serde_json::from_value(json!({ "avatar_url": "https://cdn/avatar.png" })).unwrap();

    assert_eq!(missing.avatar_url, None);
    assert_eq!(null.avatar_url, Some(None));
    assert_eq!(
        value.avatar_url,
        Some(Some("https://cdn/avatar.png".to_string()))
    );
}

#[test]
fn update_profile_input_distinguishes_missing_from_null_email() {
    let missing: UpdateProfileInput = serde_json::from_value(json!({})).unwrap();
    let null: UpdateProfileInput = serde_json::from_value(json!({ "email": null })).unwrap();
    let value: UpdateProfileInput =
        serde_json::from_value(json!({ "email": "alice@example.com" })).unwrap();

    assert_eq!(missing.email, None);
    assert_eq!(null.email, Some(None));
    assert_eq!(value.email, Some(Some("alice@example.com".to_string())));
}

#[test]
fn profile_email_update_allows_missing_email() {
    assert_eq!(profile_email_update(None).unwrap(), None);
}

#[test]
fn profile_email_update_rejects_explicit_null_email() {
    let error = profile_email_update(Some(&None)).expect_err("email null must not be a no-op");

    assert!(error.to_string().contains("email"));
}

#[test]
fn profile_email_update_keeps_email_value_for_service_validation() {
    let email = Some(" alice@example.com ".to_string());

    assert_eq!(
        profile_email_update(Some(&email)).unwrap(),
        Some(" alice@example.com ")
    );
}

#[test]
fn same_user_guard_rejects_mismatch() {
    let error = ensure_same_user(1, 2, "只能操作当前登录用户")
        .expect_err("self-service action must reject a different target user");

    assert!(error.to_string().contains("只能操作当前登录用户"));
}

#[test]
fn logout_identity_requires_user_id() {
    let error = required_logout_identity(None, Some("session-1"))
        .expect_err("logout must not silently succeed without user id");

    assert!(error.to_string().contains("用户 ID"));
}

#[test]
fn logout_identity_requires_positive_user_id() {
    let error = required_logout_identity(Some(0), Some("session-1"))
        .expect_err("logout must reject invalid user id before revoke");

    assert!(error.to_string().contains("用户 ID"));
}

#[test]
fn logout_identity_requires_session_id() {
    let error = required_logout_identity(Some(7), None)
        .expect_err("logout must not silently succeed without session id");

    assert!(error.to_string().contains("会话 ID"));
}

#[test]
fn logout_identity_rejects_blank_session_id() {
    let error = required_logout_identity(Some(7), Some("   "))
        .expect_err("logout must reject blank session id before revoke");

    assert!(error.to_string().contains("会话 ID"));
}

#[test]
fn logout_identity_trims_session_id() {
    let (user_id, session_id) =
        required_logout_identity(Some(7), Some(" session-1 ")).expect("valid logout identity");

    assert_eq!(user_id, 7);
    assert_eq!(session_id, "session-1");
}

#[test]
fn session_user_id_requires_positive_id() {
    let error = required_session_user_id(0, "列出会话")
        .expect_err("session command must reject invalid user id before service call");

    assert!(error.to_string().contains("用户 ID"));
}

#[test]
fn session_id_requires_nonblank_value() {
    let error = required_session_id("   ", "撤销会话")
        .expect_err("session command must reject blank session id before service call");

    assert!(error.to_string().contains("会话 ID"));
}

#[test]
fn session_id_is_trimmed_at_command_boundary() {
    let session_id = required_session_id(" session-1 ", "撤销会话").expect("valid session id");

    assert_eq!(session_id, "session-1");
}
