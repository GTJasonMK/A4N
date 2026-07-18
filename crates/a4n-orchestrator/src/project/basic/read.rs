use std::collections::HashMap;

use a4n_core::{error::AppError, models::project::novel_project};

use crate::OrchestratorContext;

use super::super::{
    ProjectFamilyMetadata, ProjectFamilyMetadataQuery, ProjectProgressQuery,
    ProjectProgressSummary, ProjectRollbackImpactQuery, ProjectRollbackImpactSummary,
    ProjectStatusHistorySummary, project_basic_record_error_to_app_error,
    project_family_metadata_read_error_to_app_error, project_mutation_error_to_app_error,
    project_progress_read_error_to_app_error, project_rollback_impact_read_error_to_app_error,
};

impl OrchestratorContext {
    pub async fn project_family_metadata(
        &self,
        query: ProjectFamilyMetadataQuery,
    ) -> Result<ProjectFamilyMetadata, AppError> {
        a4n_db::project::project_family_metadata(self.db.connection(), query)
            .await
            .map_err(project_family_metadata_read_error_to_app_error)
    }

    pub async fn project_family_metadata_batch(
        &self,
        queries: &[ProjectFamilyMetadataQuery],
    ) -> Result<HashMap<i64, ProjectFamilyMetadata>, AppError> {
        a4n_db::project::project_family_metadata_batch(self.db.connection(), queries)
            .await
            .map_err(project_family_metadata_read_error_to_app_error)
    }

    pub async fn project_progress_summary(
        &self,
        query: ProjectProgressQuery,
    ) -> Result<ProjectProgressSummary, AppError> {
        a4n_db::project::project_progress_summary(self.db.connection(), query)
            .await
            .map_err(project_progress_read_error_to_app_error)
    }

    pub async fn project_progress_summaries(
        &self,
        queries: &[ProjectProgressQuery],
    ) -> Result<HashMap<i64, ProjectProgressSummary>, AppError> {
        a4n_db::project::project_progress_summaries(self.db.connection(), queries)
            .await
            .map_err(project_progress_read_error_to_app_error)
    }

    pub async fn project_rollback_impacts(
        &self,
        query: ProjectRollbackImpactQuery,
    ) -> Result<Vec<ProjectRollbackImpactSummary>, AppError> {
        a4n_db::project::project_rollback_impacts(self.db.connection(), query)
            .await
            .map_err(project_rollback_impact_read_error_to_app_error)
    }

    pub async fn project_status_history(
        &self,
        project_id: i64,
    ) -> Result<Vec<ProjectStatusHistorySummary>, AppError> {
        a4n_db::project::project_status_history(self.db.connection(), project_id)
            .await
            .map_err(project_mutation_error_to_app_error)
    }

    pub async fn project_ids(&self) -> Result<Vec<i64>, AppError> {
        a4n_db::project::project_ids(self.db.connection())
            .await
            .map_err(project_mutation_error_to_app_error)
    }

    pub async fn project_kind(&self, project_id: i64) -> Result<String, AppError> {
        a4n_db::project::project_kind(self.db.connection(), project_id)
            .await
            .map_err(project_basic_record_error_to_app_error)
    }

    pub async fn project_title(&self, project_id: i64) -> Result<String, AppError> {
        a4n_db::project::project_title(self.db.connection(), project_id)
            .await
            .map_err(project_basic_record_error_to_app_error)
    }

    pub async fn project_by_id(
        &self,
        project_id: i64,
    ) -> Result<Option<novel_project::Model>, AppError> {
        a4n_db::project::project_by_id_record(self.db.connection(), project_id)
            .await
            .map_err(project_mutation_error_to_app_error)
    }
}
