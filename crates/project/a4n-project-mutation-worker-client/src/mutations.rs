use a4n_api_contract::project::{
    CreateDerivedProjectInput, CreateFreeformVideoProjectInput, CreateProjectInput,
    CreateStandaloneComicProjectInput,
    ProjectMutationApplicationPort, ProjectSummary, UpdateProjectInput,
};
use a4n_error::AppError;
use a4n_project_mutation_worker_contract::{
    CreateDerivedProjectInput as WorkerCreateDerivedProjectInput,
    CreateFreeformVideoProjectInput as WorkerCreateFreeformVideoProjectInput,
    CreateStandaloneComicProjectInput as WorkerCreateStandaloneComicProjectInput,
    CreateProjectInput as WorkerCreateProjectInput, PROJECT_MUTATION_CREATE_DERIVED_OPERATION,
    PROJECT_MUTATION_CREATE_FREEFORM_VIDEO_OPERATION, PROJECT_MUTATION_CREATE_OPERATION,
    PROJECT_MUTATION_CREATE_STANDALONE_COMIC_OPERATION,
    PROJECT_MUTATION_UPDATE_OPERATION, UpdateProjectInput as WorkerUpdateProjectInput,
};

use super::ProjectMutationWorkerClient;

#[async_trait::async_trait]
impl ProjectMutationApplicationPort for ProjectMutationWorkerClient {
    async fn create(
        &self,
        user_id: i64,
        input: CreateProjectInput,
    ) -> Result<ProjectSummary, AppError> {
        self.command(
            user_id,
            None,
            PROJECT_MUTATION_CREATE_OPERATION,
            WorkerCreateProjectInput {
                title: input.title,
                description: input.description,
                project_type: input.project_type,
            },
        )
        .await
    }

    async fn create_derived(
        &self,
        user_id: i64,
        input: CreateDerivedProjectInput,
    ) -> Result<ProjectSummary, AppError> {
        let root_project_id = Self::positive_id("root_project_id", input.root_project_id)?;
        self.command(
            user_id,
            Some(root_project_id),
            PROJECT_MUTATION_CREATE_DERIVED_OPERATION,
            WorkerCreateDerivedProjectInput {
                root_project_id,
                derived_kind: input.derived_kind,
                title: input.title,
                description: input.description,
                start_chapter: input.start_chapter,
                end_chapter: input.end_chapter,
            },
        )
        .await
    }

    async fn create_freeform_video(
        &self,
        user_id: i64,
        input: CreateFreeformVideoProjectInput,
    ) -> Result<ProjectSummary, AppError> {
        self.command(
            user_id,
            None,
            PROJECT_MUTATION_CREATE_FREEFORM_VIDEO_OPERATION,
            WorkerCreateFreeformVideoProjectInput {
                title: input.title,
                description: input.description,
            },
        )
        .await
    }

    async fn create_standalone_comic(
        &self,
        user_id: i64,
        input: CreateStandaloneComicProjectInput,
    ) -> Result<ProjectSummary, AppError> {
        self.command(
            user_id,
            None,
            PROJECT_MUTATION_CREATE_STANDALONE_COMIC_OPERATION,
            WorkerCreateStandaloneComicProjectInput {
                title: input.title,
                description: input.description,
            },
        )
        .await
    }

    async fn update(
        &self,
        user_id: i64,
        project_id: i64,
        input: UpdateProjectInput,
    ) -> Result<ProjectSummary, AppError> {
        let project_id = Self::positive_id("project_id", project_id)?;
        self.command(
            user_id,
            Some(project_id),
            PROJECT_MUTATION_UPDATE_OPERATION,
            WorkerUpdateProjectInput {
                project_id,
                title: input.title,
                description: input.description,
                fact_feedback_interval_chapters: input.fact_feedback_interval_chapters,
            },
        )
        .await
    }
}
