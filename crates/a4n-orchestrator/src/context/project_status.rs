use a4n_core::{
    error::AppError,
    event::bridge::InternalEvent,
    models::project::novel_project,
    state::{status::ProjectStatus, validator::StateValidator},
};
use a4n_db::project_reset::ProjectStatusRollbackOutcome;
use a4n_db::{DbHandle, project::UpdateProjectStatusRecordInput};
use chrono::{NaiveDateTime, Utc};

use super::OrchestratorContext;
use crate::project::{
    ProjectStatusArtifacts, ensure_text_project_status_machine_record,
    next_status_from_artifact_flags, project_mutation_error_to_app_error,
    project_reset_error_to_app_error, project_status_read_error_to_app_error,
    project_status_write_error_to_app_error, validate_forward_status_artifacts_record,
};

#[derive(Clone)]
pub(super) struct ProjectStatusApplication {
    database: DbHandle,
}

struct ProjectStatusTransition {
    project: novel_project::Model,
    from: String,
    to: String,
    changed_at: NaiveDateTime,
}

struct ProjectStatusRollback {
    outcome: ProjectStatusRollbackOutcome,
    from: String,
    to: String,
    changed_at: NaiveDateTime,
}

impl ProjectStatusApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn artifacts(&self, project_id: i64) -> Result<ProjectStatusArtifacts, AppError> {
        a4n_db::project::project_status_artifacts(self.database.connection(), project_id)
            .await
            .map_err(project_status_read_error_to_app_error)
    }

    async fn forward(
        &self,
        project_id: i64,
        target: &str,
    ) -> Result<ProjectStatusTransition, AppError> {
        let project = a4n_db::project::project_by_id_record(self.database.connection(), project_id)
            .await
            .map_err(project_mutation_error_to_app_error)?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        ensure_text_project_status_machine_record(&project)?;
        let from_status =
            StateValidator::parse_status(&project.status).map_err(|_| AppError::InvalidState {
                current: project.status.clone(),
                operation: "推进项目状态".to_string(),
            })?;
        let to_status = StateValidator::parse_status(target)?;
        StateValidator::validate_forward(&project.status, target)?;
        let artifacts = self.artifacts(project.id).await?;
        validate_forward_status_artifacts_record(&project, from_status, to_status, artifacts)?;

        let from = from_status.as_str().to_string();
        let to = to_status.as_str().to_string();
        let changed_at = Utc::now().naive_utc();
        let project = a4n_db::project::update_project_status_record(
            self.database.connection(),
            project,
            UpdateProjectStatusRecordInput {
                to_status: to.clone(),
                history_from_status: from.clone(),
                history_reason: "项目状态推进".to_string(),
                changed_by: Some("system".to_string()),
                changed_at,
                clear_draft_fields: false,
            },
        )
        .await
        .map_err(project_status_write_error_to_app_error)?;

        Ok(ProjectStatusTransition {
            project,
            from,
            to,
            changed_at,
        })
    }

    async fn rollback(
        &self,
        project_id: i64,
        target: &str,
    ) -> Result<ProjectStatusRollback, AppError> {
        let project = a4n_db::project::project_by_id_record(self.database.connection(), project_id)
            .await
            .map_err(project_mutation_error_to_app_error)?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        ensure_text_project_status_machine_record(&project)?;
        let from =
            StateValidator::parse_status(&project.status).map_err(|_| AppError::InvalidState {
                current: project.status.clone(),
                operation: "回退项目状态".to_string(),
            })?;
        let to = StateValidator::parse_status(target)?;
        StateValidator::validate_rollback(&project.status, target)?;
        let from = from.as_str().to_string();
        let to = to.as_str().to_string();
        let changed_at = Utc::now().naive_utc();
        let outcome = a4n_db::project_reset::rollback_project_status_record(
            self.database.connection(),
            project_id,
            &from,
            &to,
            changed_at,
        )
        .await
        .map_err(project_reset_error_to_app_error)?;

        Ok(ProjectStatusRollback {
            outcome,
            from,
            to,
            changed_at,
        })
    }
}

impl OrchestratorContext {
    pub(crate) async fn project_status_artifacts(
        &self,
        project_id: i64,
    ) -> Result<ProjectStatusArtifacts, AppError> {
        self.project_status.artifacts(project_id).await
    }

    pub async fn forward_project_status_record(
        &self,
        project_id: i64,
        target: &str,
    ) -> Result<novel_project::Model, AppError> {
        let transition = self.project_status.forward(project_id, target).await?;
        self.publish_internal_event(InternalEvent::ProjectStatusChanged {
            project_id,
            from: transition.from,
            to: transition.to,
            timestamp: transition.changed_at,
        })?;
        Ok(transition.project)
    }

    pub async fn rollback_project_status_record(
        &self,
        project_id: i64,
        target: &str,
    ) -> Result<ProjectStatusRollbackOutcome, AppError> {
        let rollback = self.project_status.rollback(project_id, target).await?;
        self.publish_internal_event(InternalEvent::ProjectStatusChanged {
            project_id,
            from: rollback.from,
            to: rollback.to,
            timestamp: rollback.changed_at,
        })?;
        Ok(rollback.outcome)
    }

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
