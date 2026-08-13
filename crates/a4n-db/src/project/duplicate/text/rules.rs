use crate::project::PROJECT_KIND_TEXT;
use a4n_core::models::project::novel_project;
use a4n_domain::project::ProjectStatus;

use super::super::ProjectDuplicateError;

pub(super) fn project_copy_status_includes_blueprint(branch_status: ProjectStatus) -> bool {
    branch_status != ProjectStatus::Draft
}

pub(super) fn project_copy_status_includes_part_outlines(branch_status: ProjectStatus) -> bool {
    branch_status >= ProjectStatus::PartOutlinesReady
}

pub(super) fn project_copy_status_includes_chapter_outlines(branch_status: ProjectStatus) -> bool {
    branch_status >= ProjectStatus::ChapterOutlinesReady
}

pub(super) fn project_copy_status_includes_chapters(branch_status: ProjectStatus) -> bool {
    branch_status >= ProjectStatus::Writing
}

pub(super) fn ensure_text_project_record_for_copy(
    project: &novel_project::Model,
    operation: &str,
) -> Result<(), ProjectDuplicateError> {
    if project.project_kind != PROJECT_KIND_TEXT {
        return Err(ProjectDuplicateError::InvalidArgument(format!(
            "{operation}仅支持文本项目"
        )));
    }
    Ok(())
}
