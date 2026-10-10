use super::*;

impl WorkerState {
    pub(super) async fn summary(
        &self,
        request: WorkerEnvelope<ProjectIdInput>,
    ) -> Result<ProjectSummary, WorkerFailure> {
        validate_request(&request, PROJECT_READ_SUMMARY_OPERATION, false)?;
        let user_id = request.principal.parse_user_id()?;
        let project_id = request.require_project_scope(request.payload.project_id)?;
        let project =
            a4n_db::project::project_summary_by_id_record(self.database.connection(), project_id)
                .await
                .map_err(project_error)?
                .ok_or_else(|| WorkerFailure::not_found(format!("项目不存在: {project_id}")))?;
        self.summary_for_project(user_id, project).await
    }

    pub(super) async fn list(
        &self,
        request: WorkerEnvelope<ProjectListInput>,
    ) -> Result<PaginatedProjects, WorkerFailure> {
        validate_request(&request, PROJECT_READ_LIST_OPERATION, false)?;
        let user_id = request.principal.parse_user_id()?;
        let input = request.payload;
        let offset = a4n_domain::project::project_list_offset(input.page, input.page_size)
            .map_err(WorkerFailure::invalid_request)?;
        let page = a4n_db::collaboration::list_accessible_projects(
            self.database.connection(),
            a4n_db::collaboration::AccessibleProjectListInput {
                user_id,
                offset,
                limit: input.page_size,
                project_kind: input.project_kind,
                root_project_id: input.root_project_id,
                access_scope: input
                    .access_scope
                    .unwrap_or(a4n_db::collaboration::ProjectAccessScope::All),
            },
        )
        .await
        .map_err(a4n_worker_db_adapter::map_collaboration_db_error)?;
        let projects = page
            .items
            .into_iter()
            .map(|project| project.into_summary_record())
            .collect::<Vec<_>>();
        let items = self.summary_for_projects(user_id, &projects).await?;
        Ok(PaginatedProjects {
            items,
            total: page.total,
            page: input.page,
            page_size: input.page_size,
        })
    }

    pub(super) async fn ids(
        &self,
        request: WorkerEnvelope<ProjectServiceInput>,
    ) -> Result<Vec<i64>, WorkerFailure> {
        validate_request(&request, PROJECT_READ_IDS_OPERATION, true)?;
        service_principal(&request)?;
        a4n_db::project::project_ids(self.database.connection())
            .await
            .map_err(project_error)
    }

    pub(super) async fn exists(
        &self,
        request: WorkerEnvelope<ProjectIdInput>,
    ) -> Result<bool, WorkerFailure> {
        validate_request(&request, PROJECT_READ_EXISTS_OPERATION, true)?;
        service_principal(&request)?;
        let project_id = request.require_project_scope(request.payload.project_id)?;
        a4n_db::project::project_by_id_record(self.database.connection(), project_id)
            .await
            .map(|project| project.is_some())
            .map_err(project_error)
    }

    async fn summary_for_project(
        &self,
        user_id: i64,
        project: a4n_domain::project::ProjectSummaryRecord,
    ) -> Result<ProjectSummary, WorkerFailure> {
        a4n_project_read_core::project_summary_for_user(
            self.database.connection(),
            user_id,
            &project,
        )
        .await
        .map_err(summary_error)
    }

    async fn summary_for_projects(
        &self,
        user_id: i64,
        projects: &[a4n_domain::project::ProjectSummaryRecord],
    ) -> Result<Vec<ProjectSummary>, WorkerFailure> {
        let family_queries = projects
            .iter()
            .map(a4n_db::project::ProjectFamilyMetadataQuery::from_project)
            .collect::<Vec<_>>();
        let access_queries = projects
            .iter()
            .map(a4n_db::collaboration::ProjectAccessInfoQuery::from_project)
            .collect::<Vec<_>>();
        let progress_queries = projects
            .iter()
            .map(a4n_db::project::ProjectProgressQuery::from_project)
            .collect::<Result<Vec<_>, _>>()
            .map_err(a4n_worker_db_adapter::map_project_progress_read_error)?;
        let (metadata_by_id, access_by_id, progress_by_id, intervals) = tokio::try_join!(
            async {
                a4n_db::project::project_family_metadata_batch(
                    self.database.connection(),
                    &family_queries,
                )
                .await
                .map_err(a4n_worker_db_adapter::map_project_family_metadata_read_error)
            },
            async {
                a4n_db::collaboration::project_access_info_batch(
                    self.database.connection(),
                    &access_queries,
                    user_id,
                )
                .await
                .map_err(a4n_worker_db_adapter::map_collaboration_db_error)
            },
            async {
                a4n_db::project::project_progress_summaries(
                    self.database.connection(),
                    &progress_queries,
                )
                .await
                .map_err(a4n_worker_db_adapter::map_project_progress_read_error)
            },
            async {
                a4n_db::fact_feedback::feedback_intervals(
                    self.database.connection(),
                    &projects
                        .iter()
                        .map(|project| project.id)
                        .collect::<Vec<_>>(),
                )
                .await
                .map_err(a4n_worker_db_adapter::map_database_error)
            },
        )?;
        projects
            .iter()
            .zip(&progress_queries)
            .map(|(project, query)| {
                let metadata = metadata_by_id.get(&project.id).ok_or_else(|| {
                    WorkerFailure::conflict(format!("项目缺少族元数据: {}", project.id))
                })?;
                let access = access_by_id.get(&project.id).ok_or_else(|| {
                    WorkerFailure::conflict(format!("项目缺少协作访问信息: {}", project.id))
                })?;
                let progress = progress_by_id.get(&project.id).cloned().ok_or_else(|| {
                    WorkerFailure::conflict(format!("项目缺少进度摘要: {}", project.id))
                })?;
                Ok(project_summary_from_parts(
                    project,
                    metadata,
                    access,
                    query.project_kind,
                    query.source_mode,
                    query.status,
                    progress,
                    intervals
                        .get(&project.id)
                        .copied()
                        .unwrap_or(a4n_domain::fact_feedback::DEFAULT_FEEDBACK_INTERVAL_CHAPTERS),
                ))
            })
            .collect()
    }
}
