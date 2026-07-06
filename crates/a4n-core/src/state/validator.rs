//! 项目状态运行时校验。
//!
//! 数据库加载的项目状态以字符串持久化，这里负责解析规范化值并校验状态转换。

/// 项目状态校验器
pub struct StateValidator;

impl StateValidator {
    /// 运行时校验：前进转换是否合法
    ///
    /// 作为编译时类型系统的后备，用于从数据库加载项目后验证。
    pub fn validate_forward(current: &str, target: &str) -> Result<(), crate::error::AppError> {
        use super::transitions::is_valid_forward;

        let from = Self::parse_status(current)?;
        let to = Self::parse_status(target)?;

        if is_valid_forward(from, to) {
            Ok(())
        } else {
            Err(crate::error::AppError::InvalidState {
                current: current.to_string(),
                operation: format!("前进到 {target}"),
            })
        }
    }

    /// 运行时校验：回退转换是否合法
    pub fn validate_rollback(current: &str, target: &str) -> Result<(), crate::error::AppError> {
        use super::transitions::is_valid_rollback;

        let from = Self::parse_status(current)?;
        let to = Self::parse_status(target)?;

        if is_valid_rollback(from, to) {
            Ok(())
        } else {
            Err(crate::error::AppError::InvalidState {
                current: current.to_string(),
                operation: format!("回退到 {target}"),
            })
        }
    }

    /// 解析项目状态的规范化存储值。
    pub fn parse_status(s: &str) -> Result<super::status::ProjectStatus, crate::error::AppError> {
        super::status::ProjectStatus::parse(s)
            .ok_or_else(|| crate::error::AppError::InvalidArgument(format!("未知项目状态：{s}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_forward_valid() {
        assert!(StateValidator::validate_forward("draft", "blueprint_ready").is_ok());
        assert!(StateValidator::validate_forward("writing", "completed").is_ok());
    }

    #[test]
    fn test_validate_forward_invalid() {
        assert!(StateValidator::validate_forward("draft", "writing").is_err());
        assert!(StateValidator::validate_forward("completed", "draft").is_err());
    }

    #[test]
    fn test_validate_rollback_valid() {
        assert!(StateValidator::validate_rollback("writing", "chapter_outlines_ready").is_ok());
        assert!(StateValidator::validate_rollback("completed", "draft").is_ok());
    }

    #[test]
    fn test_validate_rollback_invalid() {
        assert!(StateValidator::validate_rollback("draft", "blueprint_ready").is_err());
    }

    #[test]
    fn test_unknown_status() {
        assert!(StateValidator::validate_forward("unknown", "draft").is_err());
    }

    #[test]
    fn test_rejects_non_canonical_status_aliases() {
        assert!(StateValidator::validate_forward("Draft", "blueprint_ready").is_err());
        assert!(StateValidator::validate_forward("blueprintReady", "writing").is_err());
        assert!(StateValidator::validate_forward(" blueprint_ready ", "writing").is_err());
    }
}
