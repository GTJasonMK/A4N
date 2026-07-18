use a4n_core::error::AppError;
use serde_json::Value;

pub fn github_provider_id(user: &Value) -> Result<String, AppError> {
    user["id"]
        .as_i64()
        .map(|id| id.to_string())
        .filter(|id| !id.is_empty())
        .ok_or_else(|| AppError::InvalidArgument("GitHub 用户信息缺少数字字段 id".into()))
}

pub fn github_verified_primary_email(emails: &[Value]) -> Result<String, AppError> {
    emails
        .iter()
        .find(|email| {
            email["primary"].as_bool() == Some(true) && email["verified"].as_bool() == Some(true)
        })
        .and_then(|email| email["email"].as_str())
        .map(str::trim)
        .filter(|email| !email.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| AppError::NotFound("未找到 GitHub 已验证主邮箱".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn email_requires_verified_primary_address() {
        let emails = vec![
            json!({"email": "secondary@example.com", "primary": false, "verified": true}),
            json!({"email": "primary@example.com", "primary": true, "verified": true}),
        ];

        let email = github_verified_primary_email(&emails).expect("verified primary email");

        assert_eq!(email, "primary@example.com");
    }

    #[test]
    fn email_rejects_unverified_or_non_primary_addresses() {
        let emails = vec![
            json!({"email": "unverified@example.com", "primary": true, "verified": false}),
            json!({"email": "secondary@example.com", "primary": false, "verified": true}),
        ];

        let err = github_verified_primary_email(&emails).expect_err("email should be rejected");

        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn provider_id_requires_numeric_id() {
        let err = github_provider_id(&json!({"login": "octo"}))
            .expect_err("missing id should be rejected");

        assert!(matches!(err, AppError::InvalidArgument(_)));
    }
}
