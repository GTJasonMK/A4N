use a4n_error::AppError;

use super::super::{
    ACCESS_TOKEN_EXP_MINUTES, ACCESS_TOKEN_EXPIRES_IN_SECS, REFRESH_TOKEN_EXP_DAYS, TokenPair,
    create_access_token_with_role, create_refresh_token,
};

#[cfg(test)]
mod tests;

pub fn create_token_pair_with_role(
    user_id: &str,
    name: Option<&str>,
    role: &str,
    secret: &str,
) -> Result<TokenPair, AppError> {
    let access_token =
        create_access_token_with_role(user_id, name, role, None, secret, ACCESS_TOKEN_EXP_MINUTES)?;
    let refresh_token = create_refresh_token(user_id, secret, REFRESH_TOKEN_EXP_DAYS)?;
    Ok(TokenPair {
        access_token,
        refresh_token,
        expires_in: ACCESS_TOKEN_EXPIRES_IN_SECS,
        token_type: "Bearer".to_string(),
    })
}
