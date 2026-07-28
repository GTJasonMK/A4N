use a4n_core::{error::AppError, models::project::novel_project};

use crate::OrchestratorContext;

use super::super::{
    CreateDerivedProjectParams, CreateFreeformVideoProjectParams, CreateProjectRecordInput,
    ImportTextProjectRecordInput, ImportTextProjectRecordResult, UpdateProjectRecordInput,
    import_text_project_error_to_app_error, project_basic_record_error_to_app_error,
    project_mutation_error_to_app_error,
};

impl OrchestratorContext {
    pub async fn create_project_record(
        &self,
        input: CreateProjectRecordInput,
    ) -> Result<novel_project::Model, AppError> {
        a4n_db::project::create_project_record(self.database.connection(), input)
            .await
            .map_err(project_mutation_error_to_app_error)
    }

    pub async fn import_text_project_record(
        &self,
        input: ImportTextProjectRecordInput,
    ) -> Result<ImportTextProjectRecordResult, AppError> {
        a4n_db::project::import_text_project_record(self.database.connection(), input)
            .await
            .map_err(import_text_project_error_to_app_error)
    }

    pub async fn create_derived_project_record(
        &self,
        user_id: i64,
        params: CreateDerivedProjectParams,
    ) -> Result<novel_project::Model, AppError> {
        a4n_db::project::create_derived_project_record(self.database.connection(), user_id, params)
            .await
            .map_err(project_mutation_error_to_app_error)
    }

    pub async fn create_freeform_video_project_record(
        &self,
        user_id: i64,
        params: CreateFreeformVideoProjectParams,
    ) -> Result<novel_project::Model, AppError> {
        a4n_db::project::create_freeform_video_project_record(self.database.connection(), user_id, params)
            .await
            .map_err(project_mutation_error_to_app_error)
    }

    pub async fn update_project_record(
        &self,
        input: UpdateProjectRecordInput,
    ) -> Result<novel_project::Model, AppError> {
        a4n_db::project::update_project_record(self.database.connection(), input)
            .await
            .map_err(project_mutation_error_to_app_error)
    }

}
