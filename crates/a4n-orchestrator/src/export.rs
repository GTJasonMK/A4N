//! 导出 application：拥有导出读取、格式渲染、批处理、进度与取消语义。

mod application;

use a4n_api_contract::export::{BatchExportInput, ExportFileData};
use a4n_error::AppError;

pub use a4n_api_contract::export::ExportProgress;

/// Host-facing export boundary. Authorization, source reads and format
/// rendering remain owned by the export application; Runtime only adapts
/// command arguments and progress reporting.
#[async_trait::async_trait]
pub trait ExportApplicationPort: Send + Sync {
    async fn export_txt(&self, user_id: i64, project_id: i64) -> Result<ExportFileData, AppError>;

    async fn export_markdown(
        &self,
        user_id: i64,
        project_id: i64,
        title: &str,
        author: &str,
    ) -> Result<ExportFileData, AppError>;

    async fn export_outline_json(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ExportFileData, AppError>;

    async fn export_characters_json(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ExportFileData, AppError>;

    async fn export_epub(
        &self,
        user_id: i64,
        project_id: i64,
        title: &str,
        author: &str,
        progress: &ExportProgress<'_>,
    ) -> Result<ExportFileData, AppError>;

    async fn export_pdf(
        &self,
        user_id: i64,
        project_id: i64,
        progress: &ExportProgress<'_>,
    ) -> Result<ExportFileData, AppError>;

    async fn export_batch(
        &self,
        user_id: i64,
        input: BatchExportInput,
        progress: &ExportProgress<'_>,
    ) -> Result<Vec<ExportFileData>, AppError>;

    async fn export_galgame_webgal(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ExportFileData, AppError>;
}
