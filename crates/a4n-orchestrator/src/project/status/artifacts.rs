use a4n_core::{
    error::AppError,
    state::{status::ProjectStatus, validator::StateValidator},
};

use crate::OrchestratorContext;

use super::super::ProjectStatusArtifacts;

impl OrchestratorContext {
    pub async fn advance_project_status_if_allowed(
        &self,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<(), AppError> {
        let project = self
            .project_by_id(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        let current = StateValidator::parse_status(&project.status).map_err(|err| {
            AppError::InvalidState {
                current: project.status.clone(),
                operation: format!("自动推进项目状态到 {}: {err}", target.as_str()),
            }
        })?;

        if current.ordinal() >= target.ordinal() {
            return Ok(());
        }

        self.forward_project_status_record(project_id, target.as_str())
            .await
            .map(|_| ())
    }

    pub async fn sync_project_status_from_artifacts(
        &self,
        project_id: i64,
    ) -> Result<(), AppError> {
        loop {
            let project = self
                .project_by_id(project_id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
            let status = StateValidator::parse_status(&project.status).map_err(|err| {
                AppError::InvalidState {
                    current: project.status.clone(),
                    operation: format!("同步项目状态: {err}"),
                }
            })?;
            let artifacts = self.project_status_artifacts(project.id).await?;

            let Some(next) = next_status_from_artifact_flags(status, artifacts) else {
                break;
            };
            self.advance_project_status_if_allowed(project_id, next)
                .await?;
        }

        Ok(())
    }
}

pub(super) fn next_status_from_artifact_flags(
    status: ProjectStatus,
    artifacts: ProjectStatusArtifacts,
) -> Option<ProjectStatus> {
    match status {
        ProjectStatus::Draft => artifacts
            .has_blueprint
            .then_some(ProjectStatus::BlueprintReady),
        ProjectStatus::BlueprintReady => {
            if !artifacts.has_blueprint {
                return None;
            }
            if artifacts.has_part_outline {
                return Some(ProjectStatus::PartOutlinesReady);
            }
            artifacts
                .has_chapter_outline
                .then_some(ProjectStatus::ChapterOutlinesReady)
        }
        ProjectStatus::PartOutlinesReady => artifacts
            .has_chapter_outline
            .then_some(ProjectStatus::ChapterOutlinesReady),
        ProjectStatus::ChapterOutlinesReady => artifacts
            .has_chapter_content
            .then_some(ProjectStatus::Writing),
        ProjectStatus::Writing | ProjectStatus::Completed => None,
    }
}
