use super::*;

impl WorkerState {
    pub(super) async fn create(
        &self,
        request: WorkerEnvelope<CreateProjectInput>,
    ) -> Result<ProjectSummary, WorkerFailure> {
        unscoped_request(&request, PROJECT_MUTATION_CREATE_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        let title = required_text(&input.title, "项目标题")
            .map_err(WorkerFailure::invalid_request)?
            .to_string();
        let description = optional_text(input.description.as_deref());
        let project_type = required_text(&input.project_type, "项目类型")
            .map_err(WorkerFailure::invalid_request)?
            .to_string();
        let project = a4n_db::project::create_project_record(
            self.database.connection(),
            a4n_db::project::CreateProjectRecordInput {
                user_id,
                title,
                description,
                project_type,
            },
        )
        .await
        .map_err(project_mutation_error)?;
        self.summary_for_user(user_id, &project).await
    }

    pub(super) async fn create_derived(
        &self,
        request: WorkerEnvelope<CreateDerivedProjectInput>,
    ) -> Result<ProjectSummary, WorkerFailure> {
        let project_id = project_scope(
            &request,
            PROJECT_MUTATION_CREATE_DERIVED_OPERATION,
            request.payload.root_project_id,
        )?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        let title = required_text(&input.title, "项目标题")
            .map_err(WorkerFailure::invalid_request)?
            .to_string();
        let description = required_text(&input.description, "项目简介")
            .map_err(WorkerFailure::invalid_request)?
            .to_string();
        let root =
            a4n_db::collaboration::resolve_root_project(self.database.connection(), project_id)
                .await
                .map_err(collaboration_error)?;
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await
        .map_err(collaboration_error)?;
        let project = a4n_db::project::create_derived_project_record(
            self.database.connection(),
            root.user_id,
            a4n_db::project::CreateDerivedProjectParams {
                root_project_id: project_id,
                derived_kind: input.derived_kind,
                title,
                description,
                chapter_scope: a4n_db::project::DerivedChapterScope {
                    start_chapter: input.start_chapter,
                    end_chapter: input.end_chapter,
                },
            },
        )
        .await
        .map_err(project_mutation_error)?;
        self.summary_for_user(user_id, &project).await
    }

    pub(super) async fn create_freeform_video(
        &self,
        request: WorkerEnvelope<CreateFreeformVideoProjectInput>,
    ) -> Result<ProjectSummary, WorkerFailure> {
        unscoped_request(&request, PROJECT_MUTATION_CREATE_FREEFORM_VIDEO_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        let title = required_text(&input.title, "项目标题")
            .map_err(WorkerFailure::invalid_request)?
            .to_string();
        let description = optional_text(input.description.as_deref());
        let project = a4n_db::project::create_freeform_video_project_record(
            self.database.connection(),
            user_id,
            a4n_db::project::CreateFreeformVideoProjectParams { title, description },
        )
        .await
        .map_err(project_mutation_error)?;
        self.summary_for_user(user_id, &project).await
    }

    pub(super) async fn update(
        &self,
        request: WorkerEnvelope<UpdateProjectInput>,
    ) -> Result<ProjectSummary, WorkerFailure> {
        let project_id = project_scope(
            &request,
            PROJECT_MUTATION_UPDATE_OPERATION,
            request.payload.project_id,
        )?;
        let user_id = user_id(&request)?;
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await
        .map_err(collaboration_error)?;
        let input = request.payload;
        let title = input
            .title
            .as_deref()
            .map(|value| {
                required_text(value, "项目标题")
                    .map(str::to_string)
                    .map_err(WorkerFailure::invalid_request)
            })
            .transpose()?;
        let description = input
            .description
            .as_deref()
            .map(|value| optional_text(Some(value)));
        let project = a4n_db::project::update_project_record(
            self.database.connection(),
            a4n_db::project::UpdateProjectRecordInput {
                project_id,
                title,
                description,
                fact_feedback_interval_chapters: input.fact_feedback_interval_chapters,
            },
        )
        .await
        .map_err(project_mutation_error)?;
        self.summary_for_user(user_id, &project).await
    }

    async fn summary_for_user(
        &self,
        user_id: i64,
        project: &a4n_db_models::models::project::novel_project::Model,
    ) -> Result<ProjectSummary, WorkerFailure> {
        a4n_project_read_core::project_summary_for_user(
            self.database.connection(),
            user_id,
            project,
        )
        .await
        .map_err(summary_error)
    }
}
