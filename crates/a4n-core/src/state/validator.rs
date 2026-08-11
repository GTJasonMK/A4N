//! 项目状态运行时校验。
//!
//! 数据库加载的项目状态以字符串持久化，这里负责解析规范化值并校验状态转换。

use a4n_domain::project::ProjectStatus;

/// 解析项目状态的规范化存储值。
pub fn parse_project_status(s: &str) -> Result<ProjectStatus, crate::error::AppError> {
    ProjectStatus::parse(s)
        .ok_or_else(|| crate::error::AppError::InvalidArgument(format!("未知项目状态：{s}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_status() {
        assert_eq!(
            parse_project_status("blueprint_ready").unwrap(),
            ProjectStatus::BlueprintReady
        );
    }

    #[test]
    fn test_unknown_status() {
        assert!(parse_project_status("unknown").is_err());
    }

    #[test]
    fn test_rejects_non_canonical_status_aliases() {
        assert!(parse_project_status("Draft").is_err());
        assert!(parse_project_status("blueprintReady").is_err());
        assert!(parse_project_status(" blueprint_ready ").is_err());
    }
}
