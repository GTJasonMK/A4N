//! 认证 Host 输入的纯校验规则（无 DB）。

use a4n_core::error::AppError;

/// 调用方必须操作自身用户资源。
pub fn ensure_same_user(
    caller_user_id: i64,
    target_user_id: i64,
    message: &str,
) -> Result<(), AppError> {
    if caller_user_id == target_user_id {
        Ok(())
    } else {
        Err(AppError::Forbidden(message.to_string()))
    }
}

/// 会话操作要求正数 user_id。
pub fn required_session_user_id(user_id: i64, operation: &str) -> Result<i64, AppError> {
    if user_id > 0 {
        Ok(user_id)
    } else {
        Err(AppError::InvalidArgument(format!(
            "{operation}缺少有效用户 ID"
        )))
    }
}

/// 会话操作要求非空 session_id。
pub fn required_session_id(session_id: &str, operation: &str) -> Result<String, AppError> {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        Err(AppError::InvalidArgument(format!("{operation}缺少会话 ID")))
    } else {
        Ok(session_id.to_string())
    }
}

/// OAuth2 授权码 trim 后非空。
pub fn required_authorization_code(code: &str) -> Result<&str, AppError> {
    let code = code.trim();
    if code.is_empty() {
        Err(AppError::InvalidArgument("OAuth2 授权码不能为空".into()))
    } else {
        Ok(code)
    }
}

/// 生成 OAuth2 state（基于当前毫秒时间戳）。
pub fn oauth2_state() -> Result<String, AppError> {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .map_err(|error| AppError::Internal(anyhow::anyhow!("生成 OAuth2 state 失败：{error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_same_user_rejects_cross_user() {
        let error = ensure_same_user(1, 2, "只能操作当前登录用户").expect_err("cross");
        assert!(matches!(error, AppError::Forbidden(_)));
        assert!(ensure_same_user(1, 1, "ok").is_ok());
    }

    #[test]
    fn required_session_fields() {
        assert!(required_session_user_id(0, "列会话").is_err());
        assert_eq!(required_session_user_id(2, "列会话").unwrap(), 2);
        assert!(required_session_id("  ", "撤销").is_err());
        assert_eq!(required_session_id(" sid ", "撤销").unwrap(), "sid");
    }

    #[test]
    fn required_authorization_code_rejects_blank() {
        assert!(required_authorization_code(" \n\t ").is_err());
        assert_eq!(required_authorization_code("  abc  ").unwrap(), "abc");
    }
}
