use a4n_core::state::validator::StateValidator;
use a4n_domain::project::ProjectStatus;

use crate::project::duplicate::ProjectDuplicateError;

pub(super) fn parse_project_status(status: &str) -> Result<ProjectStatus, ProjectDuplicateError> {
    StateValidator::parse_status(status)
        .map_err(|_| ProjectDuplicateError::InvalidArgument(format!("未知项目状态：{status}")))
}

pub(super) fn ensure_branch_status_allowed(
    original_status: ProjectStatus,
    branch_status: ProjectStatus,
) -> Result<(), ProjectDuplicateError> {
    if branch_status.ordinal() > original_status.ordinal() {
        return Err(ProjectDuplicateError::InvalidState {
            current: original_status.as_str().to_string(),
            operation: format!("创建状态为 {} 的项目分支", branch_status.as_str()),
        });
    }
    Ok(())
}
