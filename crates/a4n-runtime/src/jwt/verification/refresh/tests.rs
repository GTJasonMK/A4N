use super::refresh_access_token_with_role;
use crate::jwt::{create_refresh_token, verify_token};

const TEST_SECRET: &str = "test-secret-key-for-unit-tests";

#[test]
fn rotates_refresh_token_identity() {
    let old_refresh = create_refresh_token("user-5", TEST_SECRET, 7).expect("old refresh");

    let pair = refresh_access_token_with_role(&old_refresh, None, "user", TEST_SECRET)
        .expect("refresh should create pair");

    assert_ne!(pair.refresh_token, old_refresh);
    let old_claims = verify_token(&old_refresh, TEST_SECRET).expect("old claims");
    let new_claims = verify_token(&pair.refresh_token, TEST_SECRET).expect("new claims");
    assert_eq!(new_claims.sub, old_claims.sub);
    assert_eq!(new_claims.token_type, "refresh");
    assert_ne!(new_claims.jti, old_claims.jti);
}

#[test]
fn uses_current_user_claims() {
    let old_refresh = create_refresh_token("user-6", TEST_SECRET, 7).expect("old refresh");

    let pair =
        refresh_access_token_with_role(&old_refresh, Some("只读用户"), "read_only", TEST_SECRET)
            .expect("refresh should create pair");

    let claims = verify_token(&pair.access_token, TEST_SECRET).expect("access claims");

    assert_eq!(claims.sub, "user-6");
    assert_eq!(claims.name.as_deref(), Some("只读用户"));
    assert_eq!(claims.role.as_deref(), Some("read_only"));
    assert_eq!(claims.token_type, "access");
}
