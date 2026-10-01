use a4n_db::DbHandle;
use a4n_domain::chapter::ChapterOutlineRecord;
use a4n_error::AppError;

use super::super::OrchestratorContext;
use crate::project::project_outline_record_error_to_app_error;

#[derive(Clone)]
pub(in crate::context) struct ProjectOutlineRecordsApplication {
    database: DbHandle,
}

impl ProjectOutlineRecordsApplication {
    pub(in crate::context) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn chapter_outline_by_project_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Option<ChapterOutlineRecord>, AppError> {
        a4n_db::project::chapter_outline_by_project_number(
            self.database.connection(),
            project_id,
            chapter_number,
        )
        .await
        .map_err(project_outline_record_error_to_app_error)
    }

    async fn formal_chapter_outline_by_project_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Option<ChapterOutlineRecord>, AppError> {
        a4n_db::project::formal_chapter_outline_by_project_number(
            self.database.connection(),
            project_id,
            chapter_number,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn chapter_outline_by_id(
        &self,
        outline_id: i64,
    ) -> Result<Option<ChapterOutlineRecord>, AppError> {
        a4n_db::project::chapter_outline_by_id(self.database.connection(), outline_id)
            .await
            .map_err(project_outline_record_error_to_app_error)
    }

    async fn chapter_outlines_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<ChapterOutlineRecord>, AppError> {
        a4n_db::project::chapter_outlines_for_project(self.database.connection(), project_id)
            .await
            .map_err(project_outline_record_error_to_app_error)
    }
}

impl OrchestratorContext {
    pub(crate) async fn chapter_outline_by_project_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Option<ChapterOutlineRecord>, AppError> {
        self.project_outline_records
            .chapter_outline_by_project_number(project_id, chapter_number)
            .await
    }

    pub(crate) async fn formal_chapter_outline_by_project_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Option<ChapterOutlineRecord>, AppError> {
        self.project_outline_records
            .formal_chapter_outline_by_project_number(project_id, chapter_number)
            .await
    }

    pub(crate) async fn chapter_outline_by_id(
        &self,
        outline_id: i64,
    ) -> Result<Option<ChapterOutlineRecord>, AppError> {
        self.project_outline_records
            .chapter_outline_by_id(outline_id)
            .await
    }

    pub(crate) async fn chapter_outlines_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<ChapterOutlineRecord>, AppError> {
        self.project_outline_records
            .chapter_outlines_for_project(project_id)
            .await
    }
}
