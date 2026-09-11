//! 认证 Host 输入的纯校验规则（无 DB）。

use a4n_error::AppError;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_same_user_rejects_cross_user() {
        let error = ensure_same_user(1, 2, "只能操作当前登录用户").expect_err("cross");
        assert!(matches!(error, AppError::Forbidden(_)));
        assert!(ensure_same_user(1, 1, "ok").is_ok());
    }
}
