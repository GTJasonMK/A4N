use super::*;

pub(super) fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| AppError::Internal(anyhow::anyhow!("密码哈希失败: {err}")))
}

pub(super) fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed = PasswordHash::new(hash)
        .map_err(|err| AppError::Internal(anyhow::anyhow!("密码哈希格式无效: {err}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub(super) fn hash_refresh_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

pub(super) fn normalize_username(username: &str) -> Result<String, AppError> {
    let username = username.trim();
    if username.is_empty() {
        return Err(AppError::InvalidArgument("用户名不能为空".into()));
    }
    Ok(username.to_string())
}

pub(super) fn normalize_email(email: &str) -> Result<String, AppError> {
    let email = email.trim();
    validate_email_identity(email)?;
    Ok(email.to_string())
}

pub(super) fn normalize_optional_text(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

pub(super) fn normalize_invite_code(code: &str) -> Result<String, AppError> {
    let code = code.trim();
    if code.is_empty() {
        return Err(AppError::InvalidArgument("邀请码不能为空".into()));
    }
    Ok(code.to_string())
}

pub(super) fn validate_email_identity(email: &str) -> Result<(), AppError> {
    let Some((local, domain)) = email.split_once('@') else {
        return Err(AppError::InvalidArgument("email 格式无效".into()));
    };
    if email.chars().any(char::is_whitespace)
        || local.is_empty()
        || domain.is_empty()
        || domain.contains('@')
    {
        return Err(AppError::InvalidArgument("email 格式无效".into()));
    }
    Ok(())
}

pub(super) fn validate_password_for_storage(password: &str) -> Result<(), AppError> {
    if password.trim().is_empty() {
        return Err(AppError::InvalidArgument("密码不能为空".into()));
    }
    Ok(())
}

pub(super) fn validate_positive_user_id(user_id: i64) -> Result<(), AppError> {
    if user_id <= 0 {
        Err(AppError::InvalidArgument("user_id 必须大于 0".into()))
    } else {
        Ok(())
    }
}

pub(super) fn validate_user_role(value: &str) -> Result<(), AppError> {
    a4n_domain::auth::parse_user_role(value)
        .map(|_| ())
        .map_err(AppError::InvalidArgument)
}

pub(super) fn normalize_oauth_provider(provider: &str) -> Result<String, AppError> {
    let provider = provider.trim();
    match provider {
        "" => Err(AppError::InvalidArgument("OAuth2 提供商不能为空".into())),
        "github" | "google" => Ok(provider.to_string()),
        _ => Err(AppError::InvalidArgument("不支持的 OAuth2 提供商".into())),
    }
}

pub(super) fn normalize_oauth_subject(provider_id: &str) -> Result<String, AppError> {
    let provider_id = provider_id.trim();
    if provider_id.is_empty() {
        return Err(AppError::InvalidArgument("OAuth2 用户标识不能为空".into()));
    }
    Ok(provider_id.to_string())
}

pub(super) fn required_oauth_email(email: Option<&str>) -> Result<&str, AppError> {
    let email = email
        .map(str::trim)
        .filter(|email| !email.is_empty())
        .ok_or_else(|| AppError::InvalidArgument("OAuth 用户信息缺少 email".into()))?;
    validate_email_identity(email)
        .map_err(|_| AppError::InvalidArgument("OAuth email 格式无效".into()))?;
    Ok(email)
}

pub(super) fn oauth_username_base(
    username_hint: Option<&str>,
    email: &str,
) -> Result<String, AppError> {
    if let Some(username) = username_hint
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(username.to_string());
    }
    let local = email
        .split_once('@')
        .map(|(local, _)| local.trim())
        .filter(|local| !local.is_empty())
        .ok_or_else(|| AppError::InvalidArgument("OAuth email 缺少用户名部分".into()))?;
    Ok(local.to_string())
}
