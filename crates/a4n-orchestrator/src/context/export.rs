use a4n_core::error::AppError;
use a4n_db::DbHandle;

use super::OrchestratorContext;
use crate::export::ExportChapter;

#[derive(Clone)]
pub(super) struct ExportApplication {
    database: DbHandle,
}

impl ExportApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn export_txt(&self, project_id: i64) -> Result<String, AppError> {
        a4n_db::export::export_txt(self.database.connection(), project_id).await
    }

    async fn export_markdown(
        &self,
        project_id: i64,
        title: &str,
        author: &str,
    ) -> Result<String, AppError> {
        a4n_db::export::export_markdown(self.database.connection(), project_id, title, author).await
    }

    async fn export_chapters(&self, project_id: i64) -> Result<Vec<ExportChapter>, AppError> {
        a4n_db::export::export_chapters(self.database.connection(), project_id).await
    }

    async fn export_outline_json(&self, project_id: i64) -> Result<serde_json::Value, AppError> {
        a4n_db::export::export_outline_json(self.database.connection(), project_id).await
    }

    async fn export_characters_json(&self, project_id: i64) -> Result<serde_json::Value, AppError> {
        a4n_db::export::export_characters_json(self.database.connection(), project_id).await
    }
}

impl OrchestratorContext {
    pub(crate) async fn export_txt_content(&self, project_id: i64) -> Result<String, AppError> {
        self.export.export_txt(project_id).await
    }

    pub(crate) async fn export_markdown_content(
        &self,
        project_id: i64,
        title: &str,
        author: &str,
    ) -> Result<String, AppError> {
        self.export.export_markdown(project_id, title, author).await
    }

    pub(crate) async fn export_chapter_rows(
        &self,
        project_id: i64,
    ) -> Result<Vec<ExportChapter>, AppError> {
        self.export.export_chapters(project_id).await
    }

    pub(crate) async fn export_outline_json_value(
        &self,
        project_id: i64,
    ) -> Result<serde_json::Value, AppError> {
        self.export.export_outline_json(project_id).await
    }

    pub(crate) async fn export_characters_json_value(
        &self,
        project_id: i64,
    ) -> Result<serde_json::Value, AppError> {
        self.export.export_characters_json(project_id).await
    }
}
