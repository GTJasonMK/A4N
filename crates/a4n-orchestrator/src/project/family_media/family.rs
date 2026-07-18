use a4n_core::error::AppError;

use crate::OrchestratorContext;

use super::super::{
    ChapterMangaSourceSummary, MangaCharacterPortraitRefSummary, ProjectFamilyContext,
    SourceSnapshotChapterSummary, project_family_error_to_app_error,
    project_mutation_error_to_app_error,
};

impl OrchestratorContext {
    pub async fn character_portrait_project_id(&self, portrait_id: i64) -> Result<i64, AppError> {
        a4n_db::project::character_portrait_project_id(self.db.connection(), portrait_id)
            .await
            .map_err(project_family_error_to_app_error)
    }

    pub async fn protagonist_profile_project_id(&self, profile_id: i64) -> Result<i64, AppError> {
        a4n_db::project::protagonist_profile_project_id(self.db.connection(), profile_id)
            .await
            .map_err(project_family_error_to_app_error)
    }

    pub async fn primary_material_image_ids_for_projects(
        &self,
        project_ids: &[i64],
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::project::primary_material_image_ids_for_projects(self.db.connection(), project_ids)
            .await
            .map_err(project_family_error_to_app_error)
    }

    pub async fn derived_project_family_context(
        &self,
        project_id: i64,
    ) -> Result<ProjectFamilyContext, AppError> {
        a4n_db::project::derived_project_family_context(self.db.connection(), project_id)
            .await
            .map_err(project_family_error_to_app_error)
    }

    pub async fn manga_character_portrait_refs(
        &self,
        project_id: i64,
    ) -> Result<Vec<MangaCharacterPortraitRefSummary>, AppError> {
        a4n_db::project::manga_character_portrait_refs(self.db.connection(), project_id)
            .await
            .map_err(project_family_error_to_app_error)
    }

    pub async fn chapter_manga_source_by_id(
        &self,
        chapter_id: i64,
        project_id: i64,
        require_content: bool,
    ) -> Result<ChapterMangaSourceSummary, AppError> {
        a4n_db::project::chapter_manga_source_by_id(
            self.db.connection(),
            chapter_id,
            project_id,
            require_content,
        )
        .await
        .map_err(project_family_error_to_app_error)
    }

    pub async fn chapter_manga_source_by_number(
        &self,
        project_id: i64,
        chapter_number: i32,
        require_content: bool,
    ) -> Result<ChapterMangaSourceSummary, AppError> {
        a4n_db::project::chapter_manga_source_by_number(
            self.db.connection(),
            project_id,
            chapter_number,
            require_content,
        )
        .await
        .map_err(project_family_error_to_app_error)
    }

    pub async fn source_snapshot_chapters(
        &self,
        project_id: i64,
    ) -> Result<Vec<SourceSnapshotChapterSummary>, AppError> {
        a4n_db::project::source_snapshot_chapters(self.db.connection(), project_id)
            .await
            .map_err(project_mutation_error_to_app_error)
    }
}
