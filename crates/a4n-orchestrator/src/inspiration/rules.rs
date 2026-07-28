//! 灵感对话 Host 输入的纯校验规则（无 DB）。

use a4n_core::error::AppError;

/// 历史记录 limit 必须为正。
pub fn validate_inspiration_history_limit(limit: u64) -> Result<(), AppError> {
    if limit == 0 {
        Err(AppError::InvalidArgument("limit 必须大于 0".to_string()))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_limit_must_be_positive() {
        assert!(validate_inspiration_history_limit(1).is_ok());
        let error = validate_inspiration_history_limit(0).expect_err("zero limit should fail");
        assert!(error.to_string().contains("limit 必须大于 0"));
    }
}
