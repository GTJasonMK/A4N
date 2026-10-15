use a4n_api_contract::project::{
    CreateCanvasProjectInput, CreateDerivedProjectInput, CreateProjectInput,
    CreateStandaloneComicProjectInput, ProjectMutationApplicationPort, ProjectSummary,
    UpdateProjectInput,
};
use a4n_error::AppError;
use a4n_project_mutation_worker_contract::{
    CreateCanvasProjectInput as WorkerCreateCanvasProjectInput,
    CreateDerivedProjectInput as WorkerCreateDerivedProjectInput,
    CreateProjectInput as WorkerCreateProjectInput,
    CreateStandaloneComicProjectInput as WorkerCreateStandaloneComicProjectInput,
    PROJECT_MUTATION_CREATE_CANVAS_OPERATION, PROJECT_MUTATION_CREATE_DERIVED_OPERATION,
    PROJECT_MUTATION_CREATE_OPERATION, PROJECT_MUTATION_CREATE_STANDALONE_COMIC_OPERATION,
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
        let root_project_id =
            a4n_worker_client::positive_id("root_project_id", input.root_project_id)?;
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

    async fn create_canvas(
        &self,
        user_id: i64,
        input: CreateCanvasProjectInput,
    ) -> Result<ProjectSummary, AppError> {
        self.command(
            user_id,
            None,
            PROJECT_MUTATION_CREATE_CANVAS_OPERATION,
            WorkerCreateCanvasProjectInput {
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
        let project_id = a4n_worker_client::positive_id("project_id", project_id)?;
        self.command(
            user_id,
            Some(project_id),
            PROJECT_MUTATION_UPDATE_OPERATION,
            WorkerUpdateProjectInput {
                project_id,
                title: input.title,
                description: input.description,
                fact_feedback_interval_chapters: input.fact_feedback_interval_chapters,
                expected_updated_at: input.expected_updated_at,
            },
        )
        .await
    }
}
