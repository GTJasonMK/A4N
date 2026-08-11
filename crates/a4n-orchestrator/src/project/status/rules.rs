use a4n_core::{error::AppError, models::project::novel_project};
use a4n_domain::project::ProjectStatus;

use super::super::ProjectStatusArtifacts;

pub(crate) fn validate_forward_status_artifacts_record(
    project: &novel_project::Model,
    to: ProjectStatus,
    artifacts: ProjectStatusArtifacts,
) -> Result<(), AppError> {
    let operation = match to {
        ProjectStatus::Draft => return Ok(()),
        ProjectStatus::BlueprintReady => "前进到蓝图后续阶段（缺少故事蓝图）",
        ProjectStatus::PartOutlinesReady => "前进到分部大纲后续阶段（缺少分部大纲）",
        ProjectStatus::ChapterOutlinesReady => "前进到章节大纲后续阶段（缺少章节大纲）",
        ProjectStatus::Writing => "前进到 writing（至少需要一个章节已有正文）",
        ProjectStatus::Completed => "前进到 completed（至少需要一个章节已有正文）",
    };
    if forward_status_artifacts_ready(to, artifacts) {
        Ok(())
    } else {
        Err(AppError::InvalidState {
            current: project.status.clone(),
            operation: operation.to_string(),
        })
    }
}

pub(crate) fn forward_status_artifacts_ready(
    to: ProjectStatus,
    artifacts: ProjectStatusArtifacts,
) -> bool {
    match to {
        ProjectStatus::Draft => true,
        ProjectStatus::BlueprintReady => artifacts.has_blueprint,
        ProjectStatus::PartOutlinesReady => artifacts.has_part_outline,
        ProjectStatus::ChapterOutlinesReady => artifacts.has_chapter_outline,
        ProjectStatus::Writing | ProjectStatus::Completed => artifacts.has_chapter_content,
    }
}

pub(crate) fn rollback_path(from: ProjectStatus, to: ProjectStatus) -> Vec<ProjectStatus> {
    ProjectStatus::ALL
        .into_iter()
        .rev()
        .filter(|status| status.ordinal() <= from.ordinal() && status.ordinal() >= to.ordinal())
        .collect()
}
