use a4n_core::error::AppError;

use crate::OrchestratorContext;

use super::super::{ProjectStatusArtifacts, project_status_read_error_to_app_error};

impl OrchestratorContext {
    pub async fn project_status_artifacts(
        &self,
        project_id: i64,
    ) -> Result<ProjectStatusArtifacts, AppError> {
        a4n_db::project::project_status_artifacts(self.db.connection(), project_id)
            .await
            .map_err(project_status_read_error_to_app_error)
    }
}
