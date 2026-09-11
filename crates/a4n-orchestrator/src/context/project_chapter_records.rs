use a4n_db::DbHandle;
use a4n_db_models::models::{chapter::chapter, project::source_snapshot_chapter};
use a4n_error::AppError;

use super::OrchestratorContext;
use crate::collaboration::ProjectAccessRole;
use crate::project::{
    is_character_sync_available_chapter, project_chapter_context_error_to_app_error,
    project_chapter_scene_record_error_to_app_error, project_mutation_error_to_app_error,
    validate_chapter_generation_number,
};

#[derive(Clone)]
pub(super) struct ProjectChapterRecordsApplication {
    database: DbHandle,
}

impl ProjectChapterRecordsApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn chapter_by_id(&self, chapter_id: i64) -> Result<Option<chapter::Model>, AppError> {
        a4n_db::project::chapter_by_id(self.database.connection(), chapter_id)
            .await
            .map_err(AppError::Database)
    }

    async fn chapter_by_project_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Option<chapter::Model>, AppError> {
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
    ) -> Result<Vec<chapter::Model>, AppError> {
        a4n_db::project::import_analysis_chapter_candidates(self.database.connection(), project_id)
            .await
            .map_err(project_chapter_context_error_to_app_error)
    }

    async fn character_sync_chapters(
        &self,
        project_id: i64,
    ) -> Result<Vec<chapter::Model>, AppError> {
        let chapters =
            a4n_db::project::character_sync_chapters(self.database.connection(), project_id)
                .await
                .map_err(project_chapter_context_error_to_app_error)?;
        Ok(chapters
            .into_iter()
            .filter(is_character_sync_available_chapter)
            .collect())
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
    ) -> Result<Vec<chapter::Model>, AppError> {
        a4n_db::project::chapters_before_number(
            self.database.connection(),
            project_id,
            chapter_number,
        )
        .await
        .map_err(project_chapter_context_error_to_app_error)
    }

    async fn source_snapshot_chapter_by_snapshot_and_source(
        &self,
        snapshot_id: i64,
        source_chapter_id: i64,
    ) -> Result<Option<source_snapshot_chapter::Model>, AppError> {
        a4n_db::project::source_snapshot_chapter_by_snapshot_and_source(
            self.database.connection(),
            snapshot_id,
            source_chapter_id,
        )
        .await
        .map_err(project_mutation_error_to_app_error)
    }

    async fn source_snapshot_chapter_content_by_snapshot_and_source(
        &self,
        snapshot_id: i64,
        source_chapter_id: i64,
    ) -> Result<Option<String>, AppError> {
        a4n_db::project::source_snapshot_chapter_content_by_snapshot_and_source(
            self.database.connection(),
            snapshot_id,
            source_chapter_id,
        )
        .await
        .map_err(project_mutation_error_to_app_error)
    }

    async fn chapters_for_project_numbers(
        &self,
        project_id: i64,
        chapter_numbers: &[i32],
    ) -> Result<Vec<chapter::Model>, AppError> {
        a4n_db::project::chapters_for_project_numbers(
            self.database.connection(),
            project_id,
            chapter_numbers,
        )
        .await
        .map_err(project_chapter_context_error_to_app_error)
    }
}

impl OrchestratorContext {
    pub async fn chapter_by_id(&self, chapter_id: i64) -> Result<Option<chapter::Model>, AppError> {
        self.project_chapter_records.chapter_by_id(chapter_id).await
    }

    pub(crate) async fn chapter_for_editor_validated(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<chapter::Model, AppError> {
        let chapter = self
            .chapter_by_id(chapter_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: chapter_id={chapter_id}")))?;
        self.verify_project_permission(chapter.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        Ok(chapter)
    }

    pub async fn chapter_by_project_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Option<chapter::Model>, AppError> {
        self.project_chapter_records
            .chapter_by_project_number(project_id, chapter_number)
            .await
    }

    pub(crate) async fn import_analysis_chapter_candidates(
        &self,
        project_id: i64,
    ) -> Result<Vec<chapter::Model>, AppError> {
        self.project_chapter_records
            .import_analysis_chapter_candidates(project_id)
            .await
    }

    pub async fn character_sync_chapters(
        &self,
        project_id: i64,
    ) -> Result<Vec<chapter::Model>, AppError> {
        self.project_chapter_records
            .character_sync_chapters(project_id)
            .await
    }

    pub async fn refresh_project_current_chapter(&self, project_id: i64) -> Result<(), AppError> {
        self.project_chapter_records
            .refresh_project_current_chapter(project_id)
            .await
    }

    pub async fn chapters_before_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Vec<chapter::Model>, AppError> {
        self.project_chapter_records
            .chapters_before_number(project_id, chapter_number)
            .await
    }

    pub async fn source_snapshot_chapter_by_snapshot_and_source(
        &self,
        snapshot_id: i64,
        source_chapter_id: i64,
    ) -> Result<Option<source_snapshot_chapter::Model>, AppError> {
        self.project_chapter_records
            .source_snapshot_chapter_by_snapshot_and_source(snapshot_id, source_chapter_id)
            .await
    }

    pub(crate) async fn source_snapshot_chapter_content_by_snapshot_and_source(
        &self,
        snapshot_id: i64,
        source_chapter_id: i64,
    ) -> Result<Option<String>, AppError> {
        self.project_chapter_records
            .source_snapshot_chapter_content_by_snapshot_and_source(snapshot_id, source_chapter_id)
            .await
    }

    pub async fn chapters_for_project_numbers(
        &self,
        project_id: i64,
        chapter_numbers: &[i32],
    ) -> Result<Vec<chapter::Model>, AppError> {
        self.project_chapter_records
            .chapters_for_project_numbers(project_id, chapter_numbers)
            .await
    }

    pub fn validate_chapter_generation_readiness(
        &self,
        _project_id: i64,
        chapter_number: i32,
    ) -> Result<(), String> {
        validate_chapter_generation_number(chapter_number)
    }
}
