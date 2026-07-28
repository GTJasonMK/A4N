use std::collections::HashMap;

use a4n_core::{error::AppError, models::project::novel_project};
use a4n_db::DbHandle;

use super::OrchestratorContext;
use crate::project::{
    ProjectFamilyMetadata, ProjectFamilyMetadataQuery, ProjectProgressQuery,
    ProjectProgressSummary, ProjectRollbackImpactQuery, ProjectRollbackImpactSummary,
    ProjectStatusHistorySummary, project_basic_record_error_to_app_error,
    project_family_metadata_read_error_to_app_error, project_mutation_error_to_app_error,
    project_progress_read_error_to_app_error, project_rollback_impact_read_error_to_app_error,
};

#[derive(Clone)]
pub(super) struct ProjectReadApplication {
    database: DbHandle,
}

impl ProjectReadApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn family_metadata(
        &self,
        query: ProjectFamilyMetadataQuery,
    ) -> Result<ProjectFamilyMetadata, AppError> {
        a4n_db::project::project_family_metadata(self.database.connection(), query)
            .await
            .map_err(project_family_metadata_read_error_to_app_error)
    }

    async fn family_metadata_batch(
        &self,
        queries: &[ProjectFamilyMetadataQuery],
    ) -> Result<HashMap<i64, ProjectFamilyMetadata>, AppError> {
        a4n_db::project::project_family_metadata_batch(self.database.connection(), queries)
            .await
            .map_err(project_family_metadata_read_error_to_app_error)
    }

    async fn progress_summary(
        &self,
        query: ProjectProgressQuery,
    ) -> Result<ProjectProgressSummary, AppError> {
        a4n_db::project::project_progress_summary(self.database.connection(), query)
            .await
            .map_err(project_progress_read_error_to_app_error)
    }

    async fn progress_summaries(
        &self,
        queries: &[ProjectProgressQuery],
    ) -> Result<HashMap<i64, ProjectProgressSummary>, AppError> {
        a4n_db::project::project_progress_summaries(self.database.connection(), queries)
            .await
            .map_err(project_progress_read_error_to_app_error)
    }

    async fn rollback_impacts(
        &self,
        query: ProjectRollbackImpactQuery,
    ) -> Result<Vec<ProjectRollbackImpactSummary>, AppError> {
        a4n_db::project::project_rollback_impacts(self.database.connection(), query)
            .await
            .map_err(project_rollback_impact_read_error_to_app_error)
    }

    async fn status_history(
        &self,
        project_id: i64,
    ) -> Result<Vec<ProjectStatusHistorySummary>, AppError> {
        a4n_db::project::project_status_history(self.database.connection(), project_id)
            .await
            .map_err(project_mutation_error_to_app_error)
    }

    async fn ids(&self) -> Result<Vec<i64>, AppError> {
        a4n_db::project::project_ids(self.database.connection())
            .await
            .map_err(project_mutation_error_to_app_error)
    }

    async fn kind(&self, project_id: i64) -> Result<String, AppError> {
        a4n_db::project::project_kind(self.database.connection(), project_id)
            .await
            .map_err(project_basic_record_error_to_app_error)
    }

    async fn title(&self, project_id: i64) -> Result<String, AppError> {
        a4n_db::project::project_title(self.database.connection(), project_id)
            .await
            .map_err(project_basic_record_error_to_app_error)
    }

    pub(super) async fn by_id(
        &self,
        project_id: i64,
    ) -> Result<Option<novel_project::Model>, AppError> {
        a4n_db::project::project_by_id_record(self.database.connection(), project_id)
            .await
            .map_err(project_mutation_error_to_app_error)
    }
}

impl OrchestratorContext {
    pub async fn project_family_metadata(
        &self,
        query: ProjectFamilyMetadataQuery,
    ) -> Result<ProjectFamilyMetadata, AppError> {
        self.project_read.family_metadata(query).await
    }

    pub async fn project_family_metadata_batch(
        &self,
        queries: &[ProjectFamilyMetadataQuery],
    ) -> Result<HashMap<i64, ProjectFamilyMetadata>, AppError> {
        self.project_read.family_metadata_batch(queries).await
    }

    pub async fn project_progress_summary(
        &self,
        query: ProjectProgressQuery,
    ) -> Result<ProjectProgressSummary, AppError> {
        self.project_read.progress_summary(query).await
    }

    pub async fn project_progress_summaries(
        &self,
        queries: &[ProjectProgressQuery],
    ) -> Result<HashMap<i64, ProjectProgressSummary>, AppError> {
        self.project_read.progress_summaries(queries).await
    }

    pub async fn project_rollback_impacts(
        &self,
        query: ProjectRollbackImpactQuery,
    ) -> Result<Vec<ProjectRollbackImpactSummary>, AppError> {
        self.project_read.rollback_impacts(query).await
    }

    pub async fn project_status_history(
        &self,
        project_id: i64,
    ) -> Result<Vec<ProjectStatusHistorySummary>, AppError> {
        self.project_read.status_history(project_id).await
    }

    pub async fn project_ids(&self) -> Result<Vec<i64>, AppError> {
        self.project_read.ids().await
    }

    pub async fn project_kind(&self, project_id: i64) -> Result<String, AppError> {
        self.project_read.kind(project_id).await
    }

    pub async fn project_title(&self, project_id: i64) -> Result<String, AppError> {
        self.project_read.title(project_id).await
    }

    pub async fn project_by_id(
        &self,
        project_id: i64,
    ) -> Result<Option<novel_project::Model>, AppError> {
        self.project_read.by_id(project_id).await
    }
}
