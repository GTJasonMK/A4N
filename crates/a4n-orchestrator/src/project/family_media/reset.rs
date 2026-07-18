use a4n_core::{
    error::AppError,
    event::bridge::{EventBus, InternalEvent},
    state::status::ProjectStatus,
};
use chrono::Utc;

use crate::OrchestratorContext;

use super::super::{FreeformVideoGeneratedDataResetResult, project_reset_error_to_app_error};

impl OrchestratorContext {
    /// 重置项目生成态，并在事务完成后发布唯一的状态变更事件。
    pub async fn reset_project_generated_data(
        &self,
        project_id: i64,
    ) -> Result<a4n_db::project_reset::ProjectGeneratedDataResetOutcome, AppError> {
        let outcome = a4n_db::project_reset::reset_project_generated_data_record(
            self.db.connection(),
            project_id,
        )
        .await
        .map_err(project_reset_error_to_app_error)?;
        if outcome.published_status_change {
            self.event_bus
                .publish(InternalEvent::ProjectStatusChanged {
                    project_id,
                    from: outcome.previous_status.as_str().to_string(),
                    to: ProjectStatus::Draft.as_str().to_string(),
                    timestamp: Utc::now().naive_utc(),
                })
                .await?;
        }
        Ok(outcome)
    }

    pub async fn reset_freeform_video_generated_data_records(
        &self,
        project_id: i64,
    ) -> Result<FreeformVideoGeneratedDataResetResult, AppError> {
        a4n_db::project::reset_freeform_video_generated_data_records(
            self.db.connection(),
            project_id,
        )
        .await
        .map_err(crate::freeform_generated_reset_error_to_app_error)
    }
}
