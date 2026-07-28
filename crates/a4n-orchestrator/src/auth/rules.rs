//! 认证 Host 输入的纯校验规则（无 DB）。

use a4n_core::error::AppError;
use serde::{Deserialize, Deserializer};

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

/// 资料邮箱更新：缺省表示不改；显式 null 非法；Some 取引用。
pub fn profile_email_update(email: Option<&Option<String>>) -> Result<Option<&str>, AppError> {
    match email {
        None => Ok(None),
        Some(Some(email)) => Ok(Some(email.as_str())),
        Some(None) => Err(AppError::InvalidArgument("email 不能为空".into())),
    }
}

/// 登出必须同时具备有效 user_id 与非空 session_id。
pub fn required_logout_identity(
    user_id: Option<i64>,
    session_id: Option<&str>,
) -> Result<(i64, String), AppError> {
    let user_id = user_id
        .filter(|id| *id > 0)
        .ok_or_else(|| AppError::InvalidArgument("登出缺少有效用户 ID".into()))?;
    let session_id = session_id
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| AppError::InvalidArgument("登出缺少会话 ID".into()))?;
    Ok((user_id, session_id.to_string()))
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

/// serde：区分「字段缺席」与「字段为 null」的更新语义。
pub fn deserialize_nullable_update_field<'de, D, T>(
    deserializer: D,
) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Some(Option::<T>::deserialize(deserializer)?))
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
    fn required_logout_identity_requires_both() {
        assert!(required_logout_identity(None, Some("s")).is_err());
        assert!(required_logout_identity(Some(0), Some("s")).is_err());
        assert!(required_logout_identity(Some(1), Some("  ")).is_err());
        let (uid, sid) = required_logout_identity(Some(3), Some(" abc ")).unwrap();
        assert_eq!((uid, sid.as_str()), (3, "abc"));
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
