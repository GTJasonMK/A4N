use a4n_core::{
    error::AppError,
    event::bridge::InternalEvent,
    models::project::novel_project,
    state::{
        rollback::rollback_actions,
        transitions::{is_valid_forward, is_valid_rollback, valid_forward_targets},
        validator::parse_project_status,
    },
};
use a4n_db::{
    DbHandle, project::UpdateProjectStatusRecordInput, project_reset::ProjectStatusRollbackOutcome,
};
use a4n_domain::project::{ProjectRollbackPreview, ProjectStatus, ProjectStatusMachineSummary};
use chrono::{NaiveDateTime, Utc};

use super::OrchestratorContext;
use crate::{
    collaboration::ProjectAccessRole,
    project::{
        PROJECT_KIND_TEXT, ProjectStatusArtifacts, forward_status_artifacts_ready,
        next_status_from_artifact_flags, project_mutation_error_to_app_error,
        project_reset_error_to_app_error, project_rollback_impact_read_error_to_app_error,
        project_status_read_error_to_app_error, project_status_write_error_to_app_error,
        rollback_path, validate_forward_status_artifacts_record,
    },
};

#[derive(Clone)]
pub(super) struct ProjectStatusApplication {
    database: DbHandle,
}

struct ProjectStatusTransition {
    project: novel_project::Model,
    from: ProjectStatus,
    to: ProjectStatus,
    changed_at: NaiveDateTime,
}

struct ProjectStatusRollback {
    outcome: ProjectStatusRollbackOutcome,
    from: ProjectStatus,
    to: ProjectStatus,
    changed_at: NaiveDateTime,
}

impl ProjectStatusApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn project_with_status(
        &self,
        project_id: i64,
        operation: &'static str,
    ) -> Result<(novel_project::Model, ProjectStatus), AppError> {
        let project = a4n_db::project::project_by_id_record(self.database.connection(), project_id)
            .await
            .map_err(project_mutation_error_to_app_error)?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        if project.project_kind != PROJECT_KIND_TEXT {
            return Err(AppError::InvalidArgument(
                "非文本项目不能使用文本写作状态机".to_string(),
            ));
        }
        let status = parse_project_status(&project.status).map_err(|_| AppError::InvalidState {
            current: project.status.clone(),
            operation: operation.to_string(),
        })?;
        Ok((project, status))
    }

    async fn artifacts(&self, project_id: i64) -> Result<ProjectStatusArtifacts, AppError> {
        a4n_db::project::project_status_artifacts(self.database.connection(), project_id)
            .await
            .map_err(project_status_read_error_to_app_error)
    }

    async fn machine(&self, project_id: i64) -> Result<ProjectStatusMachineSummary, AppError> {
        let (_, current_status) = self
            .project_with_status(project_id, "读取项目状态机")
            .await?;
        let artifacts = self.artifacts(project_id).await?;
        let forward_targets = valid_forward_targets(current_status)
            .iter()
            .copied()
            .filter(|target| forward_status_artifacts_ready(*target, artifacts))
            .collect();
        let rollback_targets = ProjectStatus::ALL
            .into_iter()
            .filter(|target| is_valid_rollback(current_status, *target))
            .collect();
        let history =
            a4n_db::project::project_status_history(self.database.connection(), project_id)
                .await
                .map_err(project_status_read_error_to_app_error)?;

        Ok(ProjectStatusMachineSummary {
            current_status,
            forward_targets,
            rollback_targets,
            history,
        })
    }

    async fn preview_rollback(
        &self,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<ProjectRollbackPreview, AppError> {
        let (_, current_status) = self
            .project_with_status(project_id, "预览项目状态回退")
            .await?;
        if !is_valid_rollback(current_status, target) {
            return Err(AppError::InvalidState {
                current: current_status.as_str().to_string(),
                operation: format!("回退到状态 {}", target.as_str()),
            });
        }
        let actions = rollback_actions(current_status, target);
        let impacts = a4n_db::project::project_rollback_impacts(
            self.database.connection(),
            project_id,
            &actions,
        )
        .await
        .map_err(project_rollback_impact_read_error_to_app_error)?;

        Ok(ProjectRollbackPreview {
            path: rollback_path(current_status, target),
            impacts,
        })
    }

    async fn forward(
        &self,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<ProjectStatusTransition, AppError> {
        let (project, from) = self.project_with_status(project_id, "推进项目状态").await?;
        if !is_valid_forward(from, target) {
            return Err(AppError::InvalidState {
                current: from.as_str().to_string(),
                operation: format!("前进到状态 {}", target.as_str()),
            });
        }
        let artifacts = self.artifacts(project.id).await?;
        validate_forward_status_artifacts_record(&project, target, artifacts)?;

        let changed_at = Utc::now().naive_utc();
        let project = a4n_db::project::update_project_status_record(
            self.database.connection(),
            project,
            UpdateProjectStatusRecordInput {
                to_status: target,
                history_from_status: from,
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
            to: target,
            changed_at,
        })
    }

    async fn rollback(
        &self,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<ProjectStatusRollback, AppError> {
        let (_, from) = self.project_with_status(project_id, "回退项目状态").await?;
        if !is_valid_rollback(from, target) {
            return Err(AppError::InvalidState {
                current: from.as_str().to_string(),
                operation: format!("回退到状态 {}", target.as_str()),
            });
        }
        let changed_at = Utc::now().naive_utc();
        let outcome = a4n_db::project_reset::rollback_project_status_record(
            self.database.connection(),
            project_id,
            from,
            target,
            changed_at,
        )
        .await
        .map_err(project_reset_error_to_app_error)?;

        Ok(ProjectStatusRollback {
            outcome,
            from,
            to: target,
            changed_at,
        })
    }
}

impl OrchestratorContext {
    pub async fn project_status_machine_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ProjectStatusMachineSummary, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        self.project_status.machine(project_id).await
    }

    pub async fn preview_project_status_rollback_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<ProjectRollbackPreview, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        self.project_status
            .preview_rollback(project_id, target)
            .await
    }

    pub async fn forward_project_status_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<novel_project::Model, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.forward_project_status_record(project_id, target).await
    }

    pub async fn rollback_project_status_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<novel_project::Model, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Owner)
            .await?;
        let rollback = self.project_status.rollback(project_id, target).await?;
        self.publish_internal_event(InternalEvent::ProjectStatusChanged {
            project_id,
            from: rollback.from,
            to: rollback.to,
            timestamp: rollback.changed_at,
        })?;
        if rollback.outcome.needs_rag_clear {
            self.clear_project_rag(project_id)
                .await
                .map_err(|error| AppError::VectorStoreError(error.to_string()))?;
        }
        Ok(rollback.outcome.project)
    }

    pub(crate) async fn forward_project_status_record(
        &self,
        project_id: i64,
        target: ProjectStatus,
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

    pub async fn advance_project_status_if_allowed(
        &self,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<(), AppError> {
        let project = self
            .project_by_id(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        let current =
            parse_project_status(&project.status).map_err(|err| AppError::InvalidState {
                current: project.status.clone(),
                operation: format!("自动推进项目状态到 {}: {err}", target.as_str()),
            })?;

        if current.ordinal() >= target.ordinal() {
            return Ok(());
        }

        self.forward_project_status_record(project_id, target)
            .await
            .map(|_| ())
    }

    pub async fn sync_project_status_from_artifacts(
        &self,
        project_id: i64,
    ) -> Result<(), AppError> {
        let project = self
            .project_by_id(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        let status =
            parse_project_status(&project.status).map_err(|err| AppError::InvalidState {
                current: project.status.clone(),
                operation: format!("同步项目状态: {err}"),
            })?;
        let artifacts = self.project_status.artifacts(project.id).await?;
        if let Some(next) = next_status_from_artifact_flags(status, artifacts) {
            self.advance_project_status_if_allowed(project_id, next)
                .await?;
        }
        Ok(())
    }
}
