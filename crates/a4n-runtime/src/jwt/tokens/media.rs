use a4n_core::error::AppError;
use uuid::Uuid;

use super::super::Claims;

#[cfg(test)]
mod tests;

/// 创建只读媒体访问令牌。
pub fn create_media_token(
    user_id: &str,
    media_path: &str,
    secret: &str,
    exp_minutes: u64,
) -> Result<String, AppError> {
    let media_path = media_path.trim();
    if media_path.is_empty() {
        return Err(AppError::InvalidArgument("媒体路径不能为空".into()));
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        name: None,
        role: None,
        iat: now,
        exp: now + (exp_minutes as usize * 60),
        token_type: "media".to_string(),
        jti: Some(Uuid::new_v4().to_string()),
        media_path: Some(media_path.to_string()),
    };
    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
    let key = jsonwebtoken::EncodingKey::from_secret(secret.as_bytes());
    jsonwebtoken::encode(&header, &claims, &key)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("JWT 签发失败: {e}")))
}
