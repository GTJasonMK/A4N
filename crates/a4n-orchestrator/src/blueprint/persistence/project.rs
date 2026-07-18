use a4n_core::blueprint::project_status_after_blueprint_generation;
use a4n_core::error::AppError;
use a4n_core::event::bridge::{EventBus, InternalEvent};
use a4n_core::models::project::novel_blueprint;
use a4n_core::state::validator::StateValidator;
use a4n_db::blueprint::{SyncProjectAfterBlueprintInput, sync_project_after_blueprint_record};
use chrono::Utc;
use serde_json::Value;

use crate::blueprint::{OrchestratorContext, blueprint_db_error};
use crate::project::project_status_write_error_to_app_error;

impl OrchestratorContext {
    pub(in crate::blueprint) async fn sync_project_after_blueprint_generation(
        &self,
        project_id: i64,
        blueprint: &novel_blueprint::Model,
        data: &Value,
    ) -> Result<(), AppError> {
        let project = a4n_db::project::project_by_id_record(self.db.connection(), project_id)
            .await
            .map_err(crate::project::project_mutation_error_to_app_error)?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        let previous_status = StateValidator::parse_status(&project.status)?;
        let next_status = project_status_after_blueprint_generation(previous_status);
        let now = Utc::now().naive_utc();
        let target_chapters = blueprint.total_chapters.ok_or_else(|| {
            AppError::InvalidArgument("蓝图缺少 total_chapters，无法同步项目".to_string())
        })?;
        sync_project_after_blueprint_record(
            self.db.connection(),
            SyncProjectAfterBlueprintInput {
                project_id,
                title: blueprint.title.clone(),
                status: next_status.as_str().to_string(),
                target_chapters,
                total_chapters: blueprint.total_chapters,
                chapters_per_part: blueprint.chapters_per_part,
                raw_blueprint: serde_json::to_string_pretty(data)?,
                now,
            },
        )
        .await
        .map_err(blueprint_db_error)?;
        if previous_status != next_status {
            a4n_db::project::insert_project_status_history(
                self.db.connection(),
                a4n_db::project::ProjectStatusHistoryInput {
                    project_id,
                    from_status: previous_status.as_str().to_string(),
                    to_status: next_status.as_str().to_string(),
                    reason: Some("蓝图生成完成".to_string()),
                    changed_by: Some("system".to_string()),
                    changed_at: now,
                },
            )
            .await
            .map_err(project_status_write_error_to_app_error)?;
            self.event_bus
                .publish(InternalEvent::ProjectStatusChanged {
                    project_id,
                    from: previous_status.as_str().to_string(),
                    to: next_status.as_str().to_string(),
                    timestamp: now,
                })
                .await?;
        }
        Ok(())
    }
}
