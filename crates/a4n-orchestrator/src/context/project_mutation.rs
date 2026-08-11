use a4n_core::{error::AppError, models::project::novel_project};
use a4n_db::DbHandle;

use super::OrchestratorContext;
use crate::project::{
    CreateDerivedProjectParams, CreateFreeformVideoProjectParams, CreateProjectRecordInput,
    UpdateProjectRecordInput, optional_text, project_basic_record_error_to_app_error,
    project_mutation_error_to_app_error, required_optional_text, required_text,
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

    /// 校验标题/类型后创建文本项目。
    async fn create_project_validated(
        &self,
        user_id: i64,
        title: String,
        description: Option<String>,
        project_type: Option<String>,
    ) -> Result<novel_project::Model, AppError> {
        let title = required_text(&title, "项目标题")
            .map_err(AppError::InvalidArgument)?
            .to_string();
        let description = optional_text(description.as_deref());
        let project_type = required_optional_text(project_type.as_deref(), "项目类型")
            .map_err(AppError::InvalidArgument)?
            .to_string();
        a4n_db::project::create_project_record(
            self.database.connection(),
            CreateProjectRecordInput {
                user_id,
                title,
                description,
                project_type,
            },
        )
        .await
        .map_err(project_mutation_error_to_app_error)
    }

    /// 校验标题后创建派生项目。
    async fn create_derived_project_validated(
        &self,
        owner_user_id: i64,
        mut params: CreateDerivedProjectParams,
    ) -> Result<novel_project::Model, AppError> {
        params.title = required_text(&params.title, "项目标题")
            .map_err(AppError::InvalidArgument)?
            .to_string();
        params.description = required_optional_text(Some(params.description.as_str()), "项目简介")
            .map_err(AppError::InvalidArgument)?
            .to_string();
        a4n_db::project::create_derived_project_record(
            self.database.connection(),
            owner_user_id,
            params,
        )
        .await
        .map_err(project_mutation_error_to_app_error)
    }

    /// 校验标题后创建 freeform 视频项目。
    async fn create_freeform_video_project_validated(
        &self,
        user_id: i64,
        mut params: CreateFreeformVideoProjectParams,
    ) -> Result<novel_project::Model, AppError> {
        params.title = required_text(&params.title, "项目标题")
            .map_err(AppError::InvalidArgument)?
            .to_string();
        params.description = optional_text(Some(params.description.as_str()));
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
    /// 创建文本项目：校验标题/类型后落库。
    pub async fn create_project_validated(
        &self,
        user_id: i64,
        title: String,
        description: Option<String>,
        project_type: Option<String>,
    ) -> Result<novel_project::Model, AppError> {
        self.project_mutation
            .create_project_validated(user_id, title, description, project_type)
            .await
    }

    /// 创建派生项目：校验标题/简介后落库。
    pub async fn create_derived_project_validated(
        &self,
        owner_user_id: i64,
        params: CreateDerivedProjectParams,
    ) -> Result<novel_project::Model, AppError> {
        self.project_mutation
            .create_derived_project_validated(owner_user_id, params)
            .await
    }

    /// 创建 freeform 视频项目：校验标题后落库。
    pub async fn create_freeform_video_project_validated(
        &self,
        user_id: i64,
        params: CreateFreeformVideoProjectParams,
    ) -> Result<novel_project::Model, AppError> {
        self.project_mutation
            .create_freeform_video_project_validated(user_id, params)
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
