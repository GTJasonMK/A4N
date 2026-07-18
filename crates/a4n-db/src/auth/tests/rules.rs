use a4n_core::error::AppError;

use super::super::rules::*;

#[test]
fn auth_identity_rules_trim_and_reject_invalid_values() {
    assert_eq!(normalize_username("  alice  ").unwrap(), "alice");
    assert_eq!(
        normalize_email("  alice@example.com  ").unwrap(),
        "alice@example.com"
    );
    assert!(matches!(
        normalize_username("   "),
        Err(AppError::InvalidArgument(message)) if message == "用户名不能为空"
    ));
    assert!(matches!(
        normalize_email("alice example.com"),
        Err(AppError::InvalidArgument(message)) if message == "email 格式无效"
    ));
}

#[test]
fn auth_oauth_rules_keep_provider_and_email_contracts() {
    assert_eq!(normalize_oauth_provider("github").unwrap(), "github");
    assert!(matches!(
        normalize_oauth_provider("gitlab"),
        Err(AppError::InvalidArgument(message)) if message == "不支持的 OAuth2 提供商"
    ));
    assert_eq!(
        oauth_username_base(None, "alice@example.com").unwrap(),
        "alice"
    );
    assert_eq!(
        oauth_username_base(Some(" preferred "), "alice@example.com").unwrap(),
        "preferred"
    );
}

#[test]
fn auth_password_and_refresh_token_rules_are_stable() {
    let hash = hash_password("secret").expect("password hashing should succeed");
    assert!(verify_password("secret", &hash).unwrap());
    assert!(!verify_password("wrong", &hash).unwrap());
    assert_eq!(hash_refresh_token("token"), hash_refresh_token("token"));
    assert_ne!(hash_refresh_token("token"), hash_refresh_token("other"));
    assert_eq!(hash_refresh_token("token").len(), 64);
}
