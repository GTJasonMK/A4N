use a4n_core::{error::AppError, models::project::novel_project};
use a4n_db::DbHandle;

use super::OrchestratorContext;
use crate::project::{
    CreateDerivedProjectParams, CreateFreeformVideoProjectParams, CreateProjectRecordInput,
    UpdateProjectRecordInput, optional_text, project_basic_record_error_to_app_error,
    project_mutation_error_to_app_error, required_text,
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
