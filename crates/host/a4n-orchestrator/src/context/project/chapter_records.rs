use a4n_db::DbHandle;
use a4n_error::AppError;

use super::super::OrchestratorContext;
use crate::project::{
    project_chapter_context_error_to_app_error, project_chapter_scene_record_error_to_app_error,
    project_mutation_error_to_app_error, validate_chapter_generation_number,
};
use a4n_domain::chapter::ChapterRecord;
use a4n_domain::collaboration::ProjectAccessRole;

#[derive(Clone)]
pub(in crate::context) struct ProjectChapterRecordsApplication {
    database: DbHandle,
}

impl ProjectChapterRecordsApplication {
    pub(in crate::context) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn chapter_by_id(&self, chapter_id: i64) -> Result<Option<ChapterRecord>, AppError> {
        a4n_db::project::chapter_by_id(self.database.connection(), chapter_id)
            .await
            .map_err(AppError::Database)
    }

    async fn chapter_by_project_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Option<ChapterRecord>, AppError> {
        a4n_db::project::chapter_by_project_number(
            self.database.connection(),
            project_id,
            chapter_number,
        )
        .await
        .map_err(project_chapter_scene_record_error_to_app_error)
    }

    async fn import_analysis_chapter_candidates(
        &self,
        project_id: i64,
    ) -> Result<Vec<ChapterRecord>, AppError> {
        a4n_db::project::import_analysis_chapter_candidates(self.database.connection(), project_id)
            .await
            .map_err(project_chapter_context_error_to_app_error)
    }

    async fn character_sync_chapter_records(
        &self,
        project_id: i64,
    ) -> Result<Vec<ChapterRecord>, AppError> {
        a4n_db::project::character_sync_chapter_records(self.database.connection(), project_id)
            .await
            .map_err(project_chapter_context_error_to_app_error)
    }

    async fn refresh_project_current_chapter(&self, project_id: i64) -> Result<(), AppError> {
        a4n_db::project::refresh_project_current_chapter(self.database.connection(), project_id)
            .await
            .map_err(project_chapter_context_error_to_app_error)
    }

    async fn chapters_before_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Vec<ChapterRecord>, AppError> {
        a4n_db::project::chapters_before_number(
            self.database.connection(),
            project_id,
            chapter_number,
        )
        .await
        .map_err(project_chapter_context_error_to_app_error)
    }

    async fn source_snapshot_chapter_content_by_snapshot_and_source(
        &self,
        root_project_id: i64,
        snapshot_id: i64,
        source_chapter_id: i64,
    ) -> Result<Option<String>, AppError> {
        a4n_db::project::source_snapshot_chapter_content_by_snapshot_and_source(
            self.database.connection(),
            root_project_id,
            snapshot_id,
            source_chapter_id,
        )
        .await
        .map_err(project_mutation_error_to_app_error)
    }
}

impl OrchestratorContext {
    pub(crate) async fn chapter_by_id(
        &self,
        chapter_id: i64,
    ) -> Result<Option<ChapterRecord>, AppError> {
        self.project_chapter_records.chapter_by_id(chapter_id).await
    }

    pub(crate) async fn chapter_for_editor_validated(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<ChapterRecord, AppError> {
        let chapter = self
            .chapter_by_id(chapter_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: chapter_id={chapter_id}")))?;
        self.verify_project_permission(chapter.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        Ok(chapter)
    }

    pub(crate) async fn chapter_by_project_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Option<ChapterRecord>, AppError> {
        self.project_chapter_records
            .chapter_by_project_number(project_id, chapter_number)
            .await
    }

    pub(crate) async fn import_analysis_chapter_candidates(
        &self,
        project_id: i64,
    ) -> Result<Vec<ChapterRecord>, AppError> {
        self.project_chapter_records
            .import_analysis_chapter_candidates(project_id)
            .await
    }

    pub(crate) async fn character_sync_chapter_records(
        &self,
        project_id: i64,
    ) -> Result<Vec<ChapterRecord>, AppError> {
        self.project_chapter_records
            .character_sync_chapter_records(project_id)
            .await
    }

    pub(crate) async fn refresh_project_current_chapter(
        &self,
        project_id: i64,
    ) -> Result<(), AppError> {
        self.project_chapter_records
            .refresh_project_current_chapter(project_id)
            .await
    }

    pub(crate) async fn chapters_before_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Vec<ChapterRecord>, AppError> {
        self.project_chapter_records
            .chapters_before_number(project_id, chapter_number)
            .await
    }

    pub(crate) async fn source_snapshot_chapter_content_by_snapshot_and_source(
        &self,
        root_project_id: i64,
        snapshot_id: i64,
        source_chapter_id: i64,
    ) -> Result<Option<String>, AppError> {
        self.project_chapter_records
            .source_snapshot_chapter_content_by_snapshot_and_source(
                root_project_id,
                snapshot_id,
                source_chapter_id,
            )
            .await
    }

    pub(crate) fn validate_chapter_generation_readiness(
        &self,
        _project_id: i64,
        chapter_number: i32,
    ) -> Result<(), String> {
        validate_chapter_generation_number(chapter_number)
    }
}
