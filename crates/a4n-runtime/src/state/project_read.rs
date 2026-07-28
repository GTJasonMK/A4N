use std::collections::HashMap;

use a4n_core::{error::AppError, models::project::novel_project};
use a4n_orchestrator::{
    OrchestratorContext,
    project::{
        ProjectFamilyMetadata, ProjectFamilyMetadataQuery, ProjectProgressQuery,
        ProjectProgressSummary, ProjectRollbackImpactQuery, ProjectRollbackImpactSummary,
        ProjectStatusHistorySummary,
    },
};

pub(crate) struct ProjectReadRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> ProjectReadRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn project_family_metadata(
        &self,
        query: ProjectFamilyMetadataQuery,
    ) -> Result<ProjectFamilyMetadata, AppError> {
        self.orchestrator.project_family_metadata(query).await
    }

    pub(crate) async fn project_family_metadata_batch(
        &self,
        queries: &[ProjectFamilyMetadataQuery],
    ) -> Result<HashMap<i64, ProjectFamilyMetadata>, AppError> {
        self.orchestrator
            .project_family_metadata_batch(queries)
            .await
    }

    pub(crate) async fn project_progress_summary(
        &self,
        query: ProjectProgressQuery,
    ) -> Result<ProjectProgressSummary, AppError> {
        self.orchestrator.project_progress_summary(query).await
    }

    pub(crate) async fn project_progress_summaries(
        &self,
        queries: &[ProjectProgressQuery],
    ) -> Result<HashMap<i64, ProjectProgressSummary>, AppError> {
        self.orchestrator.project_progress_summaries(queries).await
    }

    pub(crate) async fn project_rollback_impacts(
        &self,
        query: ProjectRollbackImpactQuery,
    ) -> Result<Vec<ProjectRollbackImpactSummary>, AppError> {
        self.orchestrator.project_rollback_impacts(query).await
    }

    pub(crate) async fn project_status_history(
        &self,
        project_id: i64,
    ) -> Result<Vec<ProjectStatusHistorySummary>, AppError> {
        self.orchestrator.project_status_history(project_id).await
    }

    pub(crate) async fn project_ids(&self) -> Result<Vec<i64>, AppError> {
        self.orchestrator.project_ids().await
    }

    pub(crate) async fn project_kind(&self, project_id: i64) -> Result<String, AppError> {
        self.orchestrator.project_kind(project_id).await
    }

    pub(crate) async fn project_by_id(
        &self,
        project_id: i64,
    ) -> Result<Option<novel_project::Model>, AppError> {
        self.orchestrator.project_by_id(project_id).await
    }
}
