use a4n_api_contract::project::ProjectSummary;
use a4n_db::DbHandle;
use a4n_db_models::models::project::novel_project;
use a4n_error::AppError;
use a4n_project_read_core::project_summary_from_parts;

use super::super::OrchestratorContext;
use crate::project::{
    ProjectFamilyMetadata, ProjectFamilyMetadataQuery, ProjectProgressQuery,
    ProjectProgressSummary, project_basic_record_error_to_app_error,
    project_family_metadata_read_error_to_app_error, project_mutation_error_to_app_error,
    project_progress_read_error_to_app_error,
};

#[derive(Clone)]
pub(in crate::context) struct ProjectReadApplication {
    database: DbHandle,
}

impl ProjectReadApplication {
    pub(in crate::context) fn new(database: DbHandle) -> Self {
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

    async fn progress_summary(
        &self,
        query: ProjectProgressQuery,
    ) -> Result<ProjectProgressSummary, AppError> {
        a4n_db::project::project_progress_summary(self.database.connection(), query)
            .await
            .map_err(project_progress_read_error_to_app_error)
    }

    async fn kind(&self, project_id: i64) -> Result<String, AppError> {
        a4n_db::project::project_kind(self.database.connection(), project_id)
            .await
            .map_err(project_basic_record_error_to_app_error)
    }

    pub(in crate::context) async fn by_id(
        &self,
        project_id: i64,
    ) -> Result<Option<novel_project::Model>, AppError> {
        a4n_db::project::project_by_id_record(self.database.connection(), project_id)
            .await
            .map_err(project_mutation_error_to_app_error)
    }

    pub(in crate::context) async fn summary_by_id(
        &self,
        project_id: i64,
    ) -> Result<Option<novel_project::Model>, AppError> {
        a4n_db::project::project_summary_by_id_record(self.database.connection(), project_id)
            .await
            .map_err(project_mutation_error_to_app_error)
    }
}

impl OrchestratorContext {
    pub(crate) async fn project_summary_for_user(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Option<ProjectSummary>, AppError> {
        let Some(project) = self.project_read.summary_by_id(project_id).await? else {
            return Ok(None);
        };
        let progress_query = ProjectProgressQuery::from_project(&project)
            .map_err(project_progress_read_error_to_app_error)?;
        let project_kind = progress_query.project_kind;
        let source_mode = progress_query.source_mode;
        let status = progress_query.status;
        let (progress, metadata, info) = tokio::try_join!(
            self.project_progress_summary(progress_query),
            self.project_family_metadata(ProjectFamilyMetadataQuery::from_project(&project)),
            self.project_access_info(
                a4n_db::collaboration::ProjectAccessInfoQuery::from_project(&project),
                user_id,
            ),
        )?;
        Ok(Some(project_summary_from_parts(
            &project,
            &metadata,
            &info,
            project_kind,
            source_mode,
            status,
            progress,
        )))
    }

    pub(crate) async fn project_family_metadata(
        &self,
        query: ProjectFamilyMetadataQuery,
    ) -> Result<ProjectFamilyMetadata, AppError> {
        self.project_read.family_metadata(query).await
    }

    pub(crate) async fn project_progress_summary(
        &self,
        query: ProjectProgressQuery,
    ) -> Result<ProjectProgressSummary, AppError> {
        self.project_read.progress_summary(query).await
    }

    pub(crate) async fn project_kind(&self, project_id: i64) -> Result<String, AppError> {
        self.project_read.kind(project_id).await
    }

    pub(crate) async fn project_by_id(
        &self,
        project_id: i64,
    ) -> Result<Option<novel_project::Model>, AppError> {
        self.project_read.by_id(project_id).await
    }
}
