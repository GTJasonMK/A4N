use a4n_core::error::AppError;

use super::create_access_token_with_role;
use crate::jwt::verify_token;

const TEST_SECRET: &str = "test-secret-key-for-unit-tests";

#[test]
fn creates_verifiable_access_token() {
    let token = create_access_token_with_role("user-1", Some("测试用户"), "user", TEST_SECRET, 15)
        .expect("创建令牌失败");

    let claims = verify_token(&token, TEST_SECRET).expect("验证令牌失败");

    assert_eq!(claims.sub, "user-1");
    assert_eq!(claims.token_type, "access");
    assert_eq!(claims.role.as_deref(), Some("user"));
    assert!(claims.jti.is_some());
}

#[test]
fn preserves_admin_claim() {
    let token = create_access_token_with_role("admin-1", Some("管理员"), "admin", TEST_SECRET, 15)
        .expect("创建令牌失败");

    let claims = verify_token(&token, TEST_SECRET).expect("验证令牌失败");

    assert_eq!(claims.sub, "admin-1");
    assert_eq!(claims.name.as_deref(), Some("管理员"));
    assert_eq!(claims.role.as_deref(), Some("admin"));
}

#[test]
fn rejects_blank_role() {
    let err = create_access_token_with_role("user-1", None, "   ", TEST_SECRET, 15)
        .expect_err("blank role must not be embedded in access token");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}
