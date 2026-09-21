use a4n_db_models::models::ai::optimization_session;
use a4n_error::AppError;

use super::super::OptimizeIssue;
use super::parsing::parse_optimization_issues;

pub(in crate::optimize) fn select_optimization_issues(
    session: &optimization_session::Model,
    issue_ids: Option<Vec<String>>,
) -> Result<Vec<OptimizeIssue>, AppError> {
    let issues_value = session
        .issues
        .clone()
        .ok_or_else(|| AppError::InvalidArgument("优化会话缺少 issues".to_string()))?;
    let all_issues = parse_optimization_issues(&issues_value)?;
    if all_issues.is_empty() {
        return Err(AppError::InvalidArgument(
            "没有可用于生成补丁的问题".to_string(),
        ));
    }
    if matches!(issue_ids.as_ref(), Some(ids) if ids.is_empty()) {
        return Err(AppError::InvalidArgument(
            "至少需要选择一个 issue".to_string(),
        ));
    }
    let targets = match issue_ids {
        Some(ids) => all_issues
            .into_iter()
            .filter(|issue| ids.contains(&issue.id))
            .collect::<Vec<_>>(),
        None => all_issues,
    };
    if targets.is_empty() {
        return Err(AppError::InvalidArgument(
            "选中的 issue 不存在，无法生成补丁".to_string(),
        ));
    }
    Ok(targets)
}
