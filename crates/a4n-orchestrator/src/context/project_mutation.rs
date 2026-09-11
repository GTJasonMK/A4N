use a4n_db::DbHandle;
use a4n_db_models::models::project::novel_project;
use a4n_domain::project::{optional_text, required_text};
use a4n_error::AppError;

use super::OrchestratorContext;
use crate::project::{
    CreateDerivedProjectParams, CreateFreeformVideoProjectParams, CreateProjectRecordInput,
    UpdateProjectRecordInput, project_basic_record_error_to_app_error,
    project_mutation_error_to_app_error,
};

#[derive(Clone)]
pub(super) struct ProjectMutationApplication {
    database: DbHandle,
}

impl ProjectMutationApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn import_analysis_progress(&self, project_id: i64) -> Result<Option<String>, AppError> {
        a4n_db::project::project_import_analysis_progress(self.database.connection(), project_id)
            .await
            .map_err(project_basic_record_error_to_app_error)
    }

    async fn save_import_analysis_progress(
        &self,
        project_id: i64,
        progress: String,
    ) -> Result<(), AppError> {
        a4n_db::project::save_project_import_analysis_progress(
            self.database.connection(),
            project_id,
            progress,
        )
        .await
        .map_err(project_basic_record_error_to_app_error)
    }

    async fn create_project(
        &self,
        input: CreateProjectRecordInput,
    ) -> Result<novel_project::Model, AppError> {
        a4n_db::project::create_project_record(self.database.connection(), input)
            .await
            .map_err(project_mutation_error_to_app_error)
    }

    async fn create_derived_project(
        &self,
        owner_user_id: i64,
        params: CreateDerivedProjectParams,
    ) -> Result<novel_project::Model, AppError> {
        a4n_db::project::create_derived_project_record(
            self.database.connection(),
            owner_user_id,
            params,
        )
        .await
        .map_err(project_mutation_error_to_app_error)
    }

    async fn create_freeform_video_project(
        &self,
        user_id: i64,
        params: CreateFreeformVideoProjectParams,
    ) -> Result<novel_project::Model, AppError> {
        a4n_db::project::create_freeform_video_project_record(
            self.database.connection(),
            user_id,
            params,
        )
        .await
        .map_err(project_mutation_error_to_app_error)
    }

    /// 校验可选标题后更新项目。
    async fn update_project_validated(
        &self,
        project_id: i64,
        title: Option<String>,
        description: Option<String>,
    ) -> Result<novel_project::Model, AppError> {
        let title = match title {
            Some(value) => Some(
                required_text(&value, "项目标题")
                    .map_err(AppError::InvalidArgument)?
                    .to_string(),
            ),
            None => None,
        };
        let description = description.map(|value| optional_text(Some(&value)));
        a4n_db::project::update_project_record(
            self.database.connection(),
            UpdateProjectRecordInput {
                project_id,
                title,
                description,
            },
        )
        .await
        .map_err(project_mutation_error_to_app_error)
    }
}

impl OrchestratorContext {
    pub async fn create_project(
        &self,
        input: CreateProjectRecordInput,
    ) -> Result<novel_project::Model, AppError> {
        self.project_mutation.create_project(input).await
    }

    pub async fn create_derived_project(
        &self,
        owner_user_id: i64,
        params: CreateDerivedProjectParams,
    ) -> Result<novel_project::Model, AppError> {
        self.project_mutation
            .create_derived_project(owner_user_id, params)
            .await
    }

    pub async fn create_freeform_video_project(
        &self,
        user_id: i64,
        params: CreateFreeformVideoProjectParams,
    ) -> Result<novel_project::Model, AppError> {
        self.project_mutation
            .create_freeform_video_project(user_id, params)
            .await
    }

    /// 更新项目：校验可选标题后落库。
    pub async fn update_project_validated(
        &self,
        project_id: i64,
        title: Option<String>,
        description: Option<String>,
    ) -> Result<novel_project::Model, AppError> {
        self.project_mutation
            .update_project_validated(project_id, title, description)
            .await
    }

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

#[async_trait::async_trait]
impl a4n_api_contract::project::ProjectMutationApplicationPort for OrchestratorContext {
    async fn create(
        &self,
        user_id: i64,
        input: a4n_api_contract::project::CreateProjectInput,
    ) -> Result<a4n_api_contract::project::ProjectSummary, AppError> {
        let title = required_text(&input.title, "项目标题")
            .map_err(AppError::InvalidArgument)?
            .to_string();
        let description = optional_text(input.description.as_deref());
        let project_type = required_text(&input.project_type, "项目类型")
            .map_err(AppError::InvalidArgument)?
            .to_string();
        let project = OrchestratorContext::create_project(
            self,
            CreateProjectRecordInput {
                user_id,
                title,
                description,
                project_type,
            },
        )
        .await?;
        a4n_api_contract::project::ProjectReadApplicationPort::summary(self, user_id, project.id)
            .await
    }

    async fn create_derived(
        &self,
        user_id: i64,
        input: a4n_api_contract::project::CreateDerivedProjectInput,
    ) -> Result<a4n_api_contract::project::ProjectSummary, AppError> {
        let title = required_text(&input.title, "项目标题")
            .map_err(AppError::InvalidArgument)?
            .to_string();
        let description = required_text(&input.description, "项目简介")
            .map_err(AppError::InvalidArgument)?
            .to_string();
        let root =
            OrchestratorContext::collaboration_root_project(self, input.root_project_id).await?;
        OrchestratorContext::verify_project_permission(
            self,
            input.root_project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        let project = OrchestratorContext::create_derived_project(
            self,
            root.user_id,
            CreateDerivedProjectParams {
                root_project_id: input.root_project_id,
                derived_kind: input.derived_kind,
                title,
                description,
                chapter_scope: crate::project::DerivedChapterScope {
                    start_chapter: input.start_chapter,
                    end_chapter: input.end_chapter,
                },
            },
        )
        .await?;
        a4n_api_contract::project::ProjectReadApplicationPort::summary(self, user_id, project.id)
            .await
    }

    async fn create_freeform_video(
        &self,
        user_id: i64,
        input: a4n_api_contract::project::CreateFreeformVideoProjectInput,
    ) -> Result<a4n_api_contract::project::ProjectSummary, AppError> {
        let title = required_text(&input.title, "项目标题")
            .map_err(AppError::InvalidArgument)?
            .to_string();
        let description = optional_text(input.description.as_deref());
        let project = OrchestratorContext::create_freeform_video_project(
            self,
            user_id,
            CreateFreeformVideoProjectParams { title, description },
        )
        .await?;
        a4n_api_contract::project::ProjectReadApplicationPort::summary(self, user_id, project.id)
            .await
    }

    async fn update(
        &self,
        user_id: i64,
        project_id: i64,
        input: a4n_api_contract::project::UpdateProjectInput,
    ) -> Result<a4n_api_contract::project::ProjectSummary, AppError> {
        OrchestratorContext::verify_project_permission(
            self,
            project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        OrchestratorContext::update_project_validated(
            self,
            project_id,
            input.title,
            input.description,
        )
        .await?;
        a4n_api_contract::project::ProjectReadApplicationPort::summary(self, user_id, project_id)
            .await
    }
}
