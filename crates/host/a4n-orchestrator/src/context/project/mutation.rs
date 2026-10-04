use a4n_db::DbHandle;
use a4n_error::AppError;

use super::super::OrchestratorContext;
use crate::project::project_basic_record_error_to_app_error;

#[derive(Clone)]
pub(in crate::context) struct ProjectMutationApplication {
    database: DbHandle,
}

impl ProjectMutationApplication {
    pub(in crate::context) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn import_analysis_progress(&self, project_id: i64) -> Result<Option<String>, AppError> {
        a4n_db::project::project_import_analysis_progress(self.database.executor(), project_id)
            .await
            .map_err(project_basic_record_error_to_app_error)
    }

    async fn save_import_analysis_progress(
        &self,
        project_id: i64,
        progress: String,
    ) -> Result<(), AppError> {
        a4n_db::project::save_project_import_analysis_progress(
            self.database.executor(),
            project_id,
            progress,
        )
        .await
        .map_err(project_basic_record_error_to_app_error)
    }
}

impl OrchestratorContext {
    pub(crate) async fn project_import_analysis_progress(
        &self,
        project_id: i64,
    ) -> Result<Option<String>, AppError> {
        self.project_mutation
            .import_analysis_progress(project_id)
            .await
    }

    pub(crate) async fn save_project_import_analysis_progress(
        &self,
        project_id: i64,
        progress: String,
    ) -> Result<(), AppError> {
        self.project_mutation
            .save_import_analysis_progress(project_id, progress)
            .await
    }
}
