use a4n_core::{
    error::AppError,
    event::bridge::{EventBus, InternalEvent},
    models::project::novel_project,
    state::validator::StateValidator,
};
use a4n_db::project::UpdateProjectStatusRecordInput;
use a4n_db::project_reset::ProjectStatusRollbackOutcome;
use chrono::Utc;

use crate::OrchestratorContext;

use super::super::{
    project_mutation_error_to_app_error, project_reset_error_to_app_error,
    project_status_read_error_to_app_error, project_status_write_error_to_app_error,
};
use super::rules::{
    ensure_text_project_status_machine_record, validate_forward_status_artifacts_record,
};

impl OrchestratorContext {
    pub async fn forward_project_status_record(
        &self,
        project_id: i64,
        target: &str,
    ) -> Result<novel_project::Model, AppError> {
        let project = a4n_db::project::project_by_id_record(self.db.connection(), project_id)
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
        let artifacts = a4n_db::project::project_status_artifacts(self.db.connection(), project.id)
            .await
            .map_err(project_status_read_error_to_app_error)?;
        validate_forward_status_artifacts_record(&project, from_status, to_status, artifacts)?;

        let from = from_status.as_str().to_string();
        let to = to_status.as_str().to_string();
        let changed_at = Utc::now().naive_utc();
        let updated = a4n_db::project::update_project_status_record(
            self.db.connection(),
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
        self.event_bus
            .publish(InternalEvent::ProjectStatusChanged {
                project_id,
                from,
                to,
                timestamp: changed_at,
            })
            .await?;

        Ok(updated)
    }

    pub async fn rollback_project_status_record(
        &self,
        project_id: i64,
        target: &str,
    ) -> Result<ProjectStatusRollbackOutcome, AppError> {
        let project = a4n_db::project::project_by_id_record(self.db.connection(), project_id)
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
            self.db.connection(),
            project_id,
            &from,
            &to,
            changed_at,
        )
        .await
        .map_err(project_reset_error_to_app_error)?;
        self.event_bus
            .publish(InternalEvent::ProjectStatusChanged {
                project_id,
                from,
                to,
                timestamp: changed_at,
            })
            .await?;
        Ok(outcome)
    }
}
