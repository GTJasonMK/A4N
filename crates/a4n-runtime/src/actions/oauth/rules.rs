use std::time::{SystemTime, UNIX_EPOCH};

use a4n_core::error::AppError;

pub(super) fn required_authorization_code(code: &str) -> Result<&str, AppError> {
    let code = code.trim();
    if code.is_empty() {
        Err(AppError::InvalidArgument("OAuth2 授权码不能为空".into()))
    } else {
        Ok(code)
    }
}

pub(super) fn oauth2_state() -> Result<String, AppError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .map_err(|error| AppError::Internal(anyhow::anyhow!("生成 OAuth2 state 失败：{error}")))
}
