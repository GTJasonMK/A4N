use a4n_domain::project::ProjectStatus;

use super::rules::{ensure_branch_status_allowed, parse_project_status};
use crate::project::duplicate::ProjectDuplicateError;

#[test]
fn project_status_parser_preserves_invalid_status_error() {
    assert!(matches!(
        parse_project_status("future"),
        Err(ProjectDuplicateError::InvalidArgument(message))
            if message == "未知项目状态：future"
    ));
}

#[test]
fn branch_status_cannot_advance_beyond_source() {
    assert!(ensure_branch_status_allowed(ProjectStatus::Writing, ProjectStatus::Draft).is_ok());
    assert!(ensure_branch_status_allowed(ProjectStatus::Writing, ProjectStatus::Writing).is_ok());
    assert!(matches!(
        ensure_branch_status_allowed(ProjectStatus::BlueprintReady, ProjectStatus::Writing),
        Err(ProjectDuplicateError::InvalidState { current, operation })
            if current == "blueprint_ready" && operation == "创建状态为 writing 的项目分支"
    ));
}
