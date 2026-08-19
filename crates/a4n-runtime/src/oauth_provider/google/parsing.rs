use super::super::contracts::{OAuth2UserInfo, required_non_empty_string};
use a4n_core::error::AppError;

pub fn google_oauth_user_info(user: &serde_json::Value) -> Result<OAuth2UserInfo, AppError> {
    let email = verified_google_email(user)?;
    Ok(OAuth2UserInfo {
        provider_id: required_non_empty_string(user, "sub", "Google 用户信息")?.to_string(),
        provider: "google".to_string(),
        username: Some(email.clone()),
        email: Some(email),
        avatar_url: user["picture"].as_str().map(|s| s.to_string()),
        display_name: user["name"].as_str().map(|s| s.to_string()),
    })
}

pub fn verified_google_email(user: &serde_json::Value) -> Result<String, AppError> {
    let email = required_non_empty_string(user, "email", "Google 用户信息")?.to_string();
    if user["email_verified"].as_bool() != Some(true) {
        return Err(AppError::Forbidden("Google 邮箱未验证".into()));
    }
    Ok(email)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn user_info_requires_verified_email() {
        let err = google_oauth_user_info(&json!({
            "sub": "google-1",
            "email": "victim@example.com",
            "email_verified": false
        }))
        .expect_err("unverified Google email must not be trusted for account linking");

        assert!(matches!(err, AppError::Forbidden(_)));
    }

    #[test]
    fn user_info_rejects_missing_email_verified() {
        let err = google_oauth_user_info(&json!({
            "sub": "google-1",
            "email": "victim@example.com"
        }))
        .expect_err("Google email_verified must be present and true");

        assert!(matches!(err, AppError::Forbidden(_)));
    }

    #[test]
    fn user_info_uses_verified_email_as_identity() {
        let info = google_oauth_user_info(&json!({
            "sub": "google-1",
            "email": " user@example.com ",
            "email_verified": true,
            "picture": "https://cdn.example.com/avatar.png",
            "name": "Alice"
        }))
        .expect("verified Google email should be accepted");

        assert_eq!(info.provider, "google");
        assert_eq!(info.provider_id, "google-1");
        assert_eq!(info.email.as_deref(), Some("user@example.com"));
        assert_eq!(info.username.as_deref(), Some("user@example.com"));
    }
}
