use a4n_api_contract::export::{
    BatchExportInput, ExportFileData, ExportProgress, GalgameExportApplicationPort,
    ProjectExportApplicationPort,
};
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_error::AppError;

use crate::OrchestratorContext;

#[async_trait::async_trait]
impl ProjectExportApplicationPort for OrchestratorContext {
    async fn export_txt(&self, user_id: i64, project_id: i64) -> Result<ExportFileData, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.run_export_txt_validated(project_id).await
    }

    async fn export_markdown(
        &self,
        user_id: i64,
        project_id: i64,
        title: &str,
        author: &str,
    ) -> Result<ExportFileData, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.run_export_markdown_validated(project_id, title, author)
            .await
    }

    async fn export_outline_json(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ExportFileData, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.run_export_outline_json_validated(project_id).await
    }

    async fn export_characters_json(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ExportFileData, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.run_export_characters_json_validated(project_id).await
    }

    async fn export_epub(
        &self,
        user_id: i64,
        project_id: i64,
        title: &str,
        author: &str,
        progress: &ExportProgress<'_>,
    ) -> Result<ExportFileData, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.run_export_epub_validated(project_id, title, author, progress)
            .await
    }

    async fn export_pdf(
        &self,
        user_id: i64,
        project_id: i64,
        progress: &ExportProgress<'_>,
    ) -> Result<ExportFileData, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.run_export_pdf_validated(project_id, progress).await
    }

    async fn export_batch(
        &self,
        user_id: i64,
        input: BatchExportInput,
        progress: &ExportProgress<'_>,
    ) -> Result<Vec<ExportFileData>, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.run_batch_export_validated(input, progress).await
    }
}

#[async_trait::async_trait]
impl GalgameExportApplicationPort for OrchestratorContext {
    async fn export_galgame_webgal(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ExportFileData, AppError> {
        self.run_export_galgame_webgal_for_user_validated(user_id, project_id)
            .await
    }
}
