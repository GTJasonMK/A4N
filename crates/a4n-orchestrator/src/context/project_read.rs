use std::collections::HashMap;

use a4n_api_contract::project::{PaginatedProjects, ProjectSummary};
use a4n_db::DbHandle;
use a4n_db_models::models::project::novel_project;
use a4n_domain::project::project_list_offset;
use a4n_error::AppError;

use super::OrchestratorContext;
use crate::collaboration::{AccessibleProjectListInput, ProjectAccessScope};
use crate::project::{
    ProjectFamilyMetadata, ProjectFamilyMetadataQuery, ProjectProgressQuery,
    ProjectProgressSummary, project_basic_record_error_to_app_error,
    project_family_metadata_read_error_to_app_error, project_mutation_error_to_app_error,
    project_progress_read_error_to_app_error,
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

    pub(super) async fn summary_by_id(
        &self,
        project_id: i64,
    ) -> Result<Option<novel_project::Model>, AppError> {
        a4n_db::project::project_summary_by_id_record(self.database.connection(), project_id)
            .await
            .map_err(project_mutation_error_to_app_error)
    }
}

impl OrchestratorContext {
    pub async fn project_summary_for_user(
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
                crate::collaboration::ProjectAccessInfoQuery::from_project(&project),
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

    async fn project_summaries_for_user(
        &self,
        user_id: i64,
        projects: &[novel_project::Model],
    ) -> Result<Vec<ProjectSummary>, AppError> {
        let family_queries: Vec<_> = projects
            .iter()
            .map(ProjectFamilyMetadataQuery::from_project)
            .collect();
        let access_queries: Vec<_> = projects
            .iter()
            .map(crate::collaboration::ProjectAccessInfoQuery::from_project)
            .collect();
        let progress_queries = projects
            .iter()
            .map(ProjectProgressQuery::from_project)
            .collect::<Result<Vec<_>, _>>()
            .map_err(project_progress_read_error_to_app_error)?;
        let (family_metadata_by_project_id, access_infos, progress_by_project_id) = tokio::try_join!(
            self.project_family_metadata_batch(&family_queries),
            self.project_access_info_batch(&access_queries, user_id),
            self.project_progress_summaries(&progress_queries),
        )?;

        projects
            .iter()
            .zip(&progress_queries)
            .map(|(project, query)| {
                let progress = progress_by_project_id
                    .get(&project.id)
                    .cloned()
                    .ok_or_else(|| {
                        AppError::Internal(anyhow::anyhow!(format!(
                            "项目缺少进度摘要: {}",
                            project.id
                        )))
                    })?;
                let metadata = family_metadata_by_project_id
                    .get(&project.id)
                    .ok_or_else(|| {
                        AppError::Internal(anyhow::anyhow!(format!(
                            "项目缺少族元数据: {}",
                            project.id
                        )))
                    })?;
                let info = access_infos.get(&project.id).ok_or_else(|| {
                    AppError::Internal(anyhow::anyhow!(format!(
                        "项目缺少协作访问信息: {}",
                        project.id
                    )))
                })?;
                Ok(project_summary_from_parts(
                    project,
                    metadata,
                    info,
                    query.project_kind,
                    query.source_mode,
                    query.status,
                    progress,
                ))
            })
            .collect()
    }

    async fn list_project_summaries(
        &self,
        user_id: i64,
        input: a4n_api_contract::project::ProjectListInput,
    ) -> Result<PaginatedProjects, AppError> {
        let offset =
            project_list_offset(input.page, input.page_size).map_err(AppError::InvalidArgument)?;
        let page_result = self
            .accessible_projects(AccessibleProjectListInput {
                user_id,
                offset,
                limit: input.page_size,
                project_kind: input.project_kind,
                root_project_id: input.root_project_id,
                access_scope: input.access_scope.unwrap_or(ProjectAccessScope::All),
            })
            .await?;
        let projects = page_result
            .items
            .into_iter()
            .map(|project| project.into_model())
            .collect::<Vec<_>>();
        let items = self.project_summaries_for_user(user_id, &projects).await?;
        Ok(PaginatedProjects {
            items,
            total: page_result.total,
            page: input.page,
            page_size: input.page_size,
        })
    }

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

    pub async fn project_summary_by_id(
        &self,
        project_id: i64,
    ) -> Result<Option<novel_project::Model>, AppError> {
        self.project_read.summary_by_id(project_id).await
    }
}

fn project_summary_from_parts(
    project: &novel_project::Model,
    metadata: &ProjectFamilyMetadata,
    info: &crate::collaboration::ProjectAccessInfo,
    project_kind: a4n_domain::project::ProjectKind,
    source_mode: a4n_domain::project::ProjectSourceMode,
    status: a4n_domain::project::ProjectStatus,
    progress: ProjectProgressSummary,
) -> ProjectSummary {
    ProjectSummary {
        id: project.id,
        title: project.title.clone(),
        description: project.description.clone(),
        lifecycle_gate: project.lifecycle_gate.clone(),
        project_type: project.project_type.clone(),
        project_kind,
        source_mode,
        root_project_id: project.root_project_id,
        source_sync_status: project.source_sync_status.clone(),
        root_project_title: metadata.root_project_title.clone(),
        derived_chapter_scope: metadata.derived_chapter_scope.clone(),
        status,
        current_chapter: project.current_chapter,
        target_chapters: project.target_chapters,
        total_chapters: project.total_chapters,
        chapters_per_part: project.chapters_per_part,
        owner_username: info.owner_username.clone(),
        my_role: info.my_role,
        collaboration_root_project_id: info.collaboration_root_project_id,
        member_count: info.member_count,
        pending_invite_count: info.pending_invite_count,
        progress,
        updated_at: project.updated_at.to_string(),
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::project::ProjectReadApplicationPort for OrchestratorContext {
    async fn summary(&self, user_id: i64, project_id: i64) -> Result<ProjectSummary, AppError> {
        self.project_summary_for_user(user_id, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))
    }

    async fn list(
        &self,
        user_id: i64,
        input: a4n_api_contract::project::ProjectListInput,
    ) -> Result<PaginatedProjects, AppError> {
        self.list_project_summaries(user_id, input).await
    }

    async fn source_snapshot_chapters(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<a4n_domain::project::SourceSnapshotChapterSummary>, AppError> {
        OrchestratorContext::verify_project_permission(
            self,
            project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Viewer,
        )
        .await?;
        OrchestratorContext::source_snapshot_chapters(self, project_id).await
    }

    async fn all_project_ids(&self) -> Result<Vec<i64>, AppError> {
        OrchestratorContext::project_ids(self).await
    }

    async fn exists(&self, project_id: i64) -> Result<bool, AppError> {
        Ok(OrchestratorContext::project_by_id(self, project_id)
            .await?
            .is_some())
    }
}
