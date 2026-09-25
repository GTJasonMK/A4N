use crate::oauth_provider::{GitHubOAuth2, GoogleOAuth2, OAuth2Provider};
use a4n_api_contract::auth::required_authorization_code;
use a4n_error::AppError;

use super::flow::validate_redirect_uri;
use super::provider::oauth2_provider_from_env;

#[test]
fn callback_uri_supports_app_base_paths_without_allowing_untrusted_transports() {
    assert!(validate_redirect_uri("https://example.com/a4n/auth/callback").is_ok());
    assert!(validate_redirect_uri("http://localhost:1420/auth/callback").is_ok());
    for uri in [
        "http://example.com/auth/callback",
        "https://user:secret@example.com/auth/callback",
        "https://example.com/auth/callback?next=elsewhere",
        "https://example.com/auth/callback#fragment",
    ] {
        assert!(validate_redirect_uri(uri).is_err());
    }
}

#[test]
fn authorize_url_uses_provider_specific_endpoint_and_redirect_uri() {
    let provider = GitHubOAuth2::new(
        "github-client".to_string(),
        "github-secret".to_string(),
        "http://localhost:1420/auth/callback".to_string(),
    );

    let url = provider.authorize_url("state-1", "challenge");

    assert!(url.starts_with("https://github.com/login/oauth/authorize?"));
    assert!(url.contains("client_id=github-client"));
    assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A1420%2Fauth%2Fcallback"));
    assert!(url.contains("state=state-1"));
    assert!(url.contains("code_challenge=challenge"));
    assert!(url.contains("code_challenge_method=S256"));

    let google = GoogleOAuth2::new(
        "id&extra".into(),
        "secret".into(),
        "https://app.example/auth/callback".into(),
    );
    let url = url::Url::parse(&google.authorize_url("nonce", "proof")).unwrap();
    let params: std::collections::HashMap<_, _> = url.query_pairs().collect();
    assert_eq!(params["client_id"], "id&extra");
    assert_eq!(params["redirect_uri"], "https://app.example/auth/callback");
    assert_eq!(params["code_challenge_method"], "S256");
}

#[test]
fn unsupported_provider_is_rejected_before_env_lookup() {
    let err = match oauth2_provider_from_env("wechat", "http://localhost:1420/auth/callback") {
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
