use a4n_core::error::AppError;
use uuid::Uuid;

use super::super::Claims;

#[cfg(test)]
mod tests;

pub fn create_refresh_token(
    user_id: &str,
    secret: &str,
    exp_days: u64,
) -> Result<String, AppError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        name: None,
        role: None,
        iat: now,
        exp: now + (exp_days as usize * 86400),
        token_type: "refresh".to_string(),
        jti: Some(Uuid::new_v4().to_string()),
        media_path: None,
    };
    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
    let key = jsonwebtoken::EncodingKey::from_secret(secret.as_bytes());
    jsonwebtoken::encode(&header, &claims, &key)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("JWT 签发失败: {e}")))
}
