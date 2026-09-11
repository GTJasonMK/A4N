mod refresh;

use a4n_error::AppError;

use super::Claims;
pub use refresh::refresh_access_token_with_role;

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    let key = jsonwebtoken::DecodingKey::from_secret(secret.as_bytes());
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);

    // 只验证访问令牌
    validation.set_required_spec_claims(&["exp", "sub", "iat"]);

    let token_data = jsonwebtoken::decode::<Claims>(token, &key, &validation)
        .map_err(|e| AppError::Unauthorized(format!("JWT 验证失败: {e}")))?;

    Ok(token_data.claims)
}
