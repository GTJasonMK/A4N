use a4n_core::error::AppError;
use a4n_orchestrator::{
    OrchestratorContext,
    export::{BatchExportInput, ExportFileData, ExportProgress},
};

pub(crate) struct ExportRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> ExportRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn run_export_txt_validated(
        &self,
        project_id: i64,
    ) -> Result<ExportFileData, AppError> {
        self.orchestrator.run_export_txt_validated(project_id).await
    }

    pub(crate) async fn run_export_markdown_validated(
        &self,
        project_id: i64,
        title: &str,
        author: &str,
    ) -> Result<ExportFileData, AppError> {
        self.orchestrator
            .run_export_markdown_validated(project_id, title, author)
            .await
    }

    pub(crate) async fn run_export_outline_json_validated(
        &self,
        project_id: i64,
    ) -> Result<ExportFileData, AppError> {
        self.orchestrator
            .run_export_outline_json_validated(project_id)
            .await
    }

    pub(crate) async fn run_export_characters_json_validated(
        &self,
        project_id: i64,
    ) -> Result<ExportFileData, AppError> {
        self.orchestrator
            .run_export_characters_json_validated(project_id)
            .await
    }

    pub(crate) async fn run_export_epub_validated(
        &self,
        project_id: i64,
        title: &str,
        author: &str,
        progress: &ExportProgress<'_>,
    ) -> Result<ExportFileData, AppError> {
        self.orchestrator
            .run_export_epub_validated(project_id, title, author, progress)
            .await
    }

    pub(crate) async fn run_export_pdf_validated(
        &self,
        project_id: i64,
        progress: &ExportProgress<'_>,
    ) -> Result<ExportFileData, AppError> {
        self.orchestrator
            .run_export_pdf_validated(project_id, progress)
            .await
    }

    pub(crate) async fn run_batch_export_validated(
        &self,
        input: BatchExportInput,
        progress: &ExportProgress<'_>,
    ) -> Result<Vec<ExportFileData>, AppError> {
        self.orchestrator
            .run_batch_export_validated(input, progress)
            .await
    }
}
