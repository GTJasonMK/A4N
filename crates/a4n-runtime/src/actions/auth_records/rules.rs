use a4n_core::error::AppError;
use serde::{Deserialize, Deserializer};

pub(super) fn ensure_same_user(
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

pub(super) fn profile_email_update(
    email: Option<&Option<String>>,
) -> Result<Option<&str>, AppError> {
    match email {
        None => Ok(None),
        Some(Some(email)) => Ok(Some(email.as_str())),
        Some(None) => Err(AppError::InvalidArgument("email 不能为空".into())),
    }
}

pub(super) fn required_logout_identity(
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

pub(super) fn required_session_user_id(user_id: i64, operation: &str) -> Result<i64, AppError> {
    if user_id > 0 {
        Ok(user_id)
    } else {
        Err(AppError::InvalidArgument(format!(
            "{operation}缺少有效用户 ID"
        )))
    }
}

pub(super) fn required_session_id(session_id: &str, operation: &str) -> Result<String, AppError> {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        Err(AppError::InvalidArgument(format!("{operation}缺少会话 ID")))
    } else {
        Ok(session_id.to_string())
    }
}

pub(super) fn deserialize_nullable_update_field<'de, D, T>(
    deserializer: D,
) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Some(Option::<T>::deserialize(deserializer)?))
}
