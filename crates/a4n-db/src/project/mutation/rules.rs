use super::contracts::{
    DERIVED_PROJECT_STATUS_DRAFT, PROJECT_KIND_TEXT, PROJECT_TYPE_NOVEL_LONG,
    PROJECT_TYPE_NOVEL_SHORT, PROJECT_TYPE_PROSE, PROJECT_TYPE_SCRIPT, ProjectMutationError,
    SOURCE_SYNC_CONFLICT, SOURCE_SYNC_CURRENT, SOURCE_SYNC_DETACHED, SOURCE_SYNC_ROOT_UPDATED,
};
use a4n_domain::project::ProjectKind;

const SOURCE_SYNC_STATUSES: [&str; 4] = [
    SOURCE_SYNC_CURRENT,
    SOURCE_SYNC_ROOT_UPDATED,
    SOURCE_SYNC_CONFLICT,
    SOURCE_SYNC_DETACHED,
];
const DERIVED_PROJECT_STATUSES: [&str; 1] = [DERIVED_PROJECT_STATUS_DRAFT];

pub fn is_valid_source_sync_status(value: &str) -> bool {
    SOURCE_SYNC_STATUSES.contains(&value)
}

pub fn is_valid_derived_project_status(value: &str) -> bool {
    DERIVED_PROJECT_STATUSES.contains(&value)
}

pub fn normalize_project_kind(project_kind: &str) -> Result<&str, ProjectMutationError> {
    let project_kind = project_kind.trim();
    if project_kind.is_empty() {
        return Err(ProjectMutationError::InvalidArgument(
            "项目族类型不能为空".to_string(),
        ));
    }
    ProjectKind::parse(project_kind)
        .map(|kind| kind.as_str())
        .ok_or_else(|| {
            ProjectMutationError::InvalidArgument(format!("项目族类型无效: {project_kind}"))
        })
}

pub fn normalize_derived_project_kind(project_kind: &str) -> Result<&str, ProjectMutationError> {
    let project_kind = normalize_project_kind(project_kind)?;
    if project_kind == PROJECT_KIND_TEXT {
        return Err(ProjectMutationError::InvalidArgument(
            "派生项目类型不能是 text".to_string(),
        ));
    }
    Ok(project_kind)
}

pub fn normalize_project_type(project_type: &str) -> Result<&str, ProjectMutationError> {
    let project_type = project_type.trim();
    match project_type {
        "" => Err(ProjectMutationError::InvalidArgument(
            "项目类型不能为空".to_string(),
        )),
        PROJECT_TYPE_NOVEL_LONG
        | PROJECT_TYPE_NOVEL_SHORT
        | PROJECT_TYPE_SCRIPT
        | PROJECT_TYPE_PROSE => Ok(project_type),
        other => Err(ProjectMutationError::InvalidArgument(format!(
            "项目类型无效: {other}"
        ))),
    }
}

pub(in crate::project) fn required_project_title(
    title: &str,
) -> Result<&str, ProjectMutationError> {
    let title = title.trim();
    if title.is_empty() {
        Err(ProjectMutationError::InvalidArgument(
            "项目标题不能为空".to_string(),
        ))
    } else {
        Ok(title)
    }
}
