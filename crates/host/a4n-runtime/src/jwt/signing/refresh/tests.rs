use super::create_refresh_token;
use crate::jwt::verify_token;

const TEST_SECRET: &str = "test-secret-key-for-unit-tests";

#[test]
fn tokens_have_distinct_identity_when_created_in_same_second() {
    let (first, second, first_claims, second_claims) = (0..1000)
        .find_map(|_| {
            let first = create_refresh_token("user-4", TEST_SECRET, 7).ok()?;
            let second = create_refresh_token("user-4", TEST_SECRET, 7).ok()?;
            let first_claims = verify_token(&first, TEST_SECRET).ok()?;
            let second_claims = verify_token(&second, TEST_SECRET).ok()?;
            (first_claims.iat == second_claims.iat && first_claims.exp == second_claims.exp)
                .then_some((first, second, first_claims, second_claims))
        })
        .expect("test should create two refresh tokens in the same second");

    assert_ne!(first, second);
    assert_eq!(first_claims.sub, second_claims.sub);
    assert_eq!(first_claims.exp, second_claims.exp);
    assert_ne!(first_claims.jti, second_claims.jti);
}
