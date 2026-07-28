use a4n_core::error::AppError;
use a4n_orchestrator::auth::required_authorization_code;

use super::provider::{
    DEFAULT_OAUTH2_REDIRECT_URI, oauth2_provider_from_credentials, oauth2_provider_from_env,
};

#[test]
fn authorize_url_uses_provider_specific_endpoint_and_redirect_uri() {
    let provider = oauth2_provider_from_credentials(
        "github",
        "github-client".to_string(),
        "github-secret".to_string(),
        "http://localhost:1420/auth/callback".to_string(),
    )
    .expect("provider");

    let url = provider.authorize_url("state-1");

    assert!(url.starts_with("https://github.com/login/oauth/authorize?"));
    assert!(url.contains("client_id=github-client"));
    assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A1420%2Fauth%2Fcallback"));
    assert!(url.contains("state=state-1"));
}

#[test]
fn unsupported_provider_is_rejected_before_env_lookup() {
    let err = match oauth2_provider_from_env("wechat", DEFAULT_OAUTH2_REDIRECT_URI) {
        Ok(_) => panic!("unsupported provider should fail"),
        Err(err) => err,
    };

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn authorization_code_must_not_be_blank() {
    let err = required_authorization_code(" \n\t ").expect_err("blank code should fail");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}
