use a4n_core::{error::AppError, models::project::novel_project, state::status::ProjectStatus};

use super::super::{PROJECT_KIND_TEXT, ProjectStatusArtifacts};

pub(super) fn ensure_text_project_status_machine_record(
    project: &novel_project::Model,
) -> Result<(), AppError> {
    if project.project_kind == PROJECT_KIND_TEXT {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(
            "非文本项目不能使用文本写作状态机".to_string(),
        ))
    }
}

pub(super) fn validate_forward_status_artifacts_record(
    project: &novel_project::Model,
    _from: ProjectStatus,
    to: ProjectStatus,
    artifacts: ProjectStatusArtifacts,
) -> Result<(), AppError> {
    match to {
        ProjectStatus::Draft => Ok(()),
        ProjectStatus::BlueprintReady => require_status_artifact_record(
            artifacts.has_blueprint,
            project,
            "前进到蓝图后续阶段（缺少故事蓝图）",
        ),
        ProjectStatus::PartOutlinesReady => {
            require_status_artifact_record(
                artifacts.has_blueprint,
                project,
                "前进到蓝图后续阶段（缺少故事蓝图）",
            )?;
            require_status_artifact_record(
                artifacts.has_part_outline,
                project,
                "前进到分部大纲后续阶段（缺少分部大纲）",
            )
        }
        ProjectStatus::ChapterOutlinesReady => {
            require_status_artifact_record(
                artifacts.has_blueprint,
                project,
                "前进到蓝图后续阶段（缺少故事蓝图）",
            )?;
            require_status_artifact_record(
                artifacts.has_chapter_outline,
                project,
                "前进到章节大纲后续阶段（缺少章节大纲）",
            )
        }
        ProjectStatus::Writing => {
            require_status_artifact_record(
                artifacts.has_blueprint,
                project,
                "前进到蓝图后续阶段（缺少故事蓝图）",
            )?;
            require_status_artifact_record(
                artifacts.has_chapter_outline,
                project,
                "前进到章节大纲后续阶段（缺少章节大纲）",
            )?;
            require_status_artifact_record(
                artifacts.has_chapter_content,
                project,
                "前进到 writing（至少需要一个章节已有正文）",
            )
        }
        ProjectStatus::Completed => {
            require_status_artifact_record(
                artifacts.has_blueprint,
                project,
                "前进到蓝图后续阶段（缺少故事蓝图）",
            )?;
            require_status_artifact_record(
                artifacts.has_chapter_content,
                project,
                "前进到 completed（至少需要一个章节已有正文）",
            )
        }
    }
}

fn require_status_artifact_record(
    exists: bool,
    project: &novel_project::Model,
    operation: &'static str,
) -> Result<(), AppError> {
    if exists {
        Ok(())
    } else {
        Err(invalid_forward_status_record(project, operation))
    }
}

fn invalid_forward_status_record(
    project: &novel_project::Model,
    operation: &'static str,
) -> AppError {
    AppError::InvalidState {
        current: project.status.clone(),
        operation: operation.to_string(),
    }
}
