use crate::OrchestratorContext;
use a4n_core::{error::AppError, models::project::novel_project};

impl OrchestratorContext {
    pub async fn detach_derived_project_record(
        &self,
        project: novel_project::Model,
    ) -> Result<novel_project::Model, AppError> {
        a4n_db::source_sync::detach_derived_project_record(self.db.connection(), project)
            .await
            .map_err(super::source_sync_db_error)
    }
}
