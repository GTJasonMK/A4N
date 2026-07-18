use a4n_core::{error::AppError, models::project::novel_project};
use chrono::Utc;
use sea_orm::TransactionTrait;

use crate::OrchestratorContext;

use super::super::{
    DerivedProjectMediaFileCopier, FreeformProjectMediaFileCopier,
    project_duplicate_error_to_app_error,
};

impl OrchestratorContext {
    pub async fn duplicate_text_project_record(
        &self,
        original: &novel_project::Model,
        user_id: i64,
        new_title: &str,
    ) -> Result<novel_project::Model, AppError> {
        a4n_db::project::duplicate_text_project_record(
            self.db.connection(),
            original,
            user_id,
            new_title,
        )
        .await
        .map_err(project_duplicate_error_to_app_error)
    }

    pub async fn branch_text_project_record(
        &self,
        original: &novel_project::Model,
        branch_title: &str,
        branch_status: &str,
    ) -> Result<novel_project::Model, AppError> {
        a4n_db::project::branch_text_project_record(
            self.db.connection(),
            original,
            branch_title,
            branch_status,
        )
        .await
        .map_err(project_duplicate_error_to_app_error)
    }

    pub async fn duplicate_derived_project_record<Copier>(
        &self,
        original: &novel_project::Model,
        user_id: i64,
        new_title: &str,
        media_file_copier: &mut Copier,
    ) -> Result<novel_project::Model, AppError>
    where
        Copier: DerivedProjectMediaFileCopier,
    {
        a4n_db::project::duplicate_derived_project_record(
            self.db.connection(),
            original,
            user_id,
            new_title,
            media_file_copier,
        )
        .await
        .map_err(project_duplicate_error_to_app_error)
    }

    pub async fn duplicate_freeform_project_record<Copier>(
        &self,
        original: &novel_project::Model,
        user_id: i64,
        new_title: &str,
        media_file_copier: &mut Copier,
    ) -> Result<novel_project::Model, AppError>
    where
        Copier: FreeformProjectMediaFileCopier,
    {
        let tx = self
            .db
            .connection()
            .begin()
            .await
            .map_err(AppError::Database)?;
        let now = Utc::now().naive_utc();
        let project = a4n_db::project::duplicate_freeform_project_record(
            &tx,
            original,
            user_id,
            new_title,
            now,
            media_file_copier,
        )
        .await
        .map_err(project_duplicate_error_to_app_error)?;
        tx.commit().await.map_err(AppError::Database)?;
        Ok(project)
    }
}
