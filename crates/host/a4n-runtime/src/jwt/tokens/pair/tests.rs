use super::create_token_pair_with_role;
use crate::jwt::{ACCESS_TOKEN_EXPIRES_IN_SECS, verify_token};

const TEST_SECRET: &str = "test-secret-key-for-unit-tests";

#[test]
fn creates_bearer_pair() {
    let pair =
        create_token_pair_with_role("user-3", None, "user", TEST_SECRET).expect("创建令牌对失败");

    assert_eq!(pair.token_type, "Bearer");
    assert_eq!(pair.expires_in, ACCESS_TOKEN_EXPIRES_IN_SECS);
    assert!(!pair.access_token.is_empty());
    assert!(!pair.refresh_token.is_empty());
}

#[test]
fn preserves_user_claim() {
    let pair = create_token_pair_with_role("user-1", Some("作者"), "user", TEST_SECRET)
        .expect("创建令牌对失败");

    let claims = verify_token(&pair.access_token, TEST_SECRET).expect("验证令牌失败");

    assert_eq!(claims.sub, "user-1");
    assert_eq!(claims.name.as_deref(), Some("作者"));
    assert_eq!(claims.role.as_deref(), Some("user"));
}
