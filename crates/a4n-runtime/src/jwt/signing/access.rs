use a4n_core::error::AppError;
use uuid::Uuid;

use super::super::Claims;

#[cfg(test)]
mod tests;

pub fn create_access_token_with_role(
    user_id: &str,
    name: Option<&str>,
    role: &str,
    session_id: Option<&str>,
    secret: &str,
    exp_minutes: u64,
) -> Result<String, AppError> {
    let role = role_claim(role)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        name: name.map(|s| s.to_string()),
        role: Some(role),
        iat: now,
        exp: now + (exp_minutes as usize * 60),
        token_type: "access".to_string(),
        jti: Some(Uuid::new_v4().to_string()),
        session_id: session_id.map(str::to_string),
        media_path: None,
    };
    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
    let key = jsonwebtoken::EncodingKey::from_secret(secret.as_bytes());
    jsonwebtoken::encode(&header, &claims, &key)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("JWT 签发失败: {e}")))
}

fn role_claim(role: &str) -> Result<String, AppError> {
    if role.trim().is_empty() {
        return Err(AppError::InvalidArgument("用户角色不能为空".into()));
    }
    Ok(role.to_string())
}
