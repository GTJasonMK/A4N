use super::super::{
    chapter, project_chapter_context_error_to_app_error,
    project_chapter_scene_record_error_to_app_error, project_mutation_error_to_app_error,
    source_snapshot_chapter,
};
use crate::OrchestratorContext;
use a4n_core::error::AppError;

impl OrchestratorContext {
    pub async fn chapter_by_id(&self, chapter_id: i64) -> Result<Option<chapter::Model>, AppError> {
        a4n_db::project::chapter_by_id(self.db.connection(), chapter_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn chapter_by_project_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Option<chapter::Model>, AppError> {
        a4n_db::project::chapter_by_project_number(self.db.connection(), project_id, chapter_number)
            .await
            .map_err(project_chapter_scene_record_error_to_app_error)
    }

    pub async fn import_analysis_chapter_candidates(
        &self,
        project_id: i64,
    ) -> Result<Vec<chapter::Model>, AppError> {
        a4n_db::project::import_analysis_chapter_candidates(self.db.connection(), project_id)
            .await
            .map_err(project_chapter_context_error_to_app_error)
    }

    pub async fn character_sync_chapters(
        &self,
        project_id: i64,
    ) -> Result<Vec<chapter::Model>, AppError> {
        let chapters = a4n_db::project::character_sync_chapters(self.db.connection(), project_id)
            .await
            .map_err(project_chapter_context_error_to_app_error)?;
        Ok(chapters
            .into_iter()
            .filter(is_character_sync_available_chapter)
            .collect())
    }

    pub async fn refresh_project_current_chapter(&self, project_id: i64) -> Result<(), AppError> {
        a4n_db::project::refresh_project_current_chapter(self.db.connection(), project_id)
            .await
            .map_err(project_chapter_context_error_to_app_error)
    }

    pub async fn chapters_before_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Vec<chapter::Model>, AppError> {
        a4n_db::project::chapters_before_number(self.db.connection(), project_id, chapter_number)
            .await
            .map_err(project_chapter_context_error_to_app_error)
    }

    pub async fn source_snapshot_chapter_by_snapshot_and_source(
        &self,
        snapshot_id: i64,
        source_chapter_id: i64,
    ) -> Result<Option<source_snapshot_chapter::Model>, AppError> {
        a4n_db::project::source_snapshot_chapter_by_snapshot_and_source(
            self.db.connection(),
            snapshot_id,
            source_chapter_id,
        )
        .await
        .map_err(project_mutation_error_to_app_error)
    }

    pub async fn chapters_for_project_numbers(
        &self,
        project_id: i64,
        chapter_numbers: &[i32],
    ) -> Result<Vec<chapter::Model>, AppError> {
        a4n_db::project::chapters_for_project_numbers(
            self.db.connection(),
            project_id,
            chapter_numbers,
        )
        .await
        .map_err(project_chapter_context_error_to_app_error)
    }
}

pub(super) fn is_character_sync_available_chapter(chapter: &chapter::Model) -> bool {
    chapter.has_available_content()
}
