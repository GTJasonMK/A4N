use a4n_core::error::AppError;

use super::super::{
    ACCESS_TOKEN_EXP_MINUTES, ACCESS_TOKEN_EXPIRES_IN_SECS, REFRESH_TOKEN_EXP_DAYS, TokenPair,
    create_access_token_with_role, create_refresh_token,
};
use super::verify_token;

#[cfg(test)]
mod tests;

/// 使用刷新令牌和当前用户信息获取新的令牌对。
pub fn refresh_access_token_with_role(
    refresh_token: &str,
    name: Option<&str>,
    role: &str,
    secret: &str,
) -> Result<TokenPair, AppError> {
    let claims = verify_token(refresh_token, secret)?;
    if claims.token_type != "refresh" {
        return Err(AppError::Unauthorized("令牌类型错误：需要刷新令牌".into()));
    }
    let access_token = create_access_token_with_role(
        &claims.sub,
        name.or(claims.name.as_deref()),
        role,
        secret,
        ACCESS_TOKEN_EXP_MINUTES,
    )?;
    let new_refresh_token = create_refresh_token(&claims.sub, secret, REFRESH_TOKEN_EXP_DAYS)?;
    Ok(TokenPair {
        access_token,
        refresh_token: new_refresh_token,
        expires_in: ACCESS_TOKEN_EXPIRES_IN_SECS,
        token_type: "Bearer".to_string(),
    })
}
