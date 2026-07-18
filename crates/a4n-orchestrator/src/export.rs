pub use a4n_db::export::{ExportChapter, chapter_heading};

use a4n_core::error::AppError;

use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn export_txt(&self, project_id: i64) -> Result<String, AppError> {
        a4n_db::export::export_txt(self.db.connection(), project_id).await
    }

    pub async fn export_markdown(
        &self,
        project_id: i64,
        title: &str,
        author: &str,
    ) -> Result<String, AppError> {
        a4n_db::export::export_markdown(self.db.connection(), project_id, title, author).await
    }

    pub async fn export_chapters(&self, project_id: i64) -> Result<Vec<ExportChapter>, AppError> {
        a4n_db::export::export_chapters(self.db.connection(), project_id).await
    }

    pub async fn export_outline_json(
        &self,
        project_id: i64,
    ) -> Result<serde_json::Value, AppError> {
        a4n_db::export::export_outline_json(self.db.connection(), project_id).await
    }

    pub async fn export_characters_json(
        &self,
        project_id: i64,
    ) -> Result<serde_json::Value, AppError> {
        a4n_db::export::export_characters_json(self.db.connection(), project_id).await
    }
}
