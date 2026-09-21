use super::create_media_token;
use crate::jwt::verify_token;

const TEST_SECRET: &str = "test-secret-key-for-unit-tests";

#[test]
fn binds_single_media_path() {
    let token = create_media_token(
        "user-7",
        "projects/1/images/chapter_1/frame.png",
        TEST_SECRET,
        5,
    )
    .expect("media token should be created");

    let claims = verify_token(&token, TEST_SECRET).expect("media token should verify");

    assert_eq!(claims.sub, "user-7");
    assert_eq!(claims.token_type, "media");
    assert_eq!(
        claims.media_path.as_deref(),
        Some("projects/1/images/chapter_1/frame.png")
    );
    assert!(claims.jti.is_some());
}
