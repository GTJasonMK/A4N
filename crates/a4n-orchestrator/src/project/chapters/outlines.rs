use a4n_db::project::{
    ImportBlueprintUpsertInput, ImportChapterOutlineUpsertInput, ImportPartOutlineUpsertInput,
    OutlineGenerationProgressRows, ProjectChapterOutlineUpdateInput,
};

use super::super::errors::{
    project_chapter_outline_update_error_to_app_error, project_outline_record_error_to_app_error,
};
use super::super::{chapter_outline, novel_blueprint, part_outline};
use crate::OrchestratorContext;
use a4n_core::error::AppError;

impl OrchestratorContext {
    pub async fn chapter_outline_by_project_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Option<chapter_outline::Model>, AppError> {
        a4n_db::project::chapter_outline_by_project_number(
            self.db.connection(),
            project_id,
            chapter_number,
        )
        .await
        .map_err(project_outline_record_error_to_app_error)
    }

    pub async fn chapter_outline_by_id(
        &self,
        outline_id: i64,
    ) -> Result<Option<chapter_outline::Model>, AppError> {
        a4n_db::project::chapter_outline_by_id(self.db.connection(), outline_id)
            .await
            .map_err(project_outline_record_error_to_app_error)
    }

    pub async fn upsert_import_chapter_outline(
        &self,
        input: ImportChapterOutlineUpsertInput,
    ) -> Result<chapter_outline::Model, AppError> {
        a4n_db::project::upsert_import_chapter_outline(self.db.connection(), input)
            .await
            .map_err(project_outline_record_error_to_app_error)
    }

    pub async fn update_chapter_outline_record(
        &self,
        input: ProjectChapterOutlineUpdateInput,
    ) -> Result<chapter_outline::Model, a4n_core::error::AppError> {
        a4n_db::project::update_chapter_outline_record(self.db.connection(), input)
            .await
            .map_err(project_chapter_outline_update_error_to_app_error)
    }

    pub async fn upsert_import_part_outline(
        &self,
        input: ImportPartOutlineUpsertInput,
    ) -> Result<part_outline::Model, AppError> {
        a4n_db::project::upsert_import_part_outline(self.db.connection(), input)
            .await
            .map_err(project_outline_record_error_to_app_error)
    }

    pub async fn upsert_import_blueprint(
        &self,
        input: ImportBlueprintUpsertInput,
    ) -> Result<novel_blueprint::Model, AppError> {
        a4n_db::project::upsert_import_blueprint(self.db.connection(), input)
            .await
            .map_err(project_outline_record_error_to_app_error)
    }

    pub async fn chapter_outlines_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        a4n_db::project::chapter_outlines_for_project(self.db.connection(), project_id)
            .await
            .map_err(project_outline_record_error_to_app_error)
    }

    pub async fn part_outlines_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<part_outline::Model>, AppError> {
        a4n_db::project::part_outlines_for_project(self.db.connection(), project_id)
            .await
            .map_err(project_outline_record_error_to_app_error)
    }

    pub async fn part_outline_by_id(
        &self,
        part_id: i64,
    ) -> Result<Option<part_outline::Model>, AppError> {
        a4n_db::project::part_outline_by_id(self.db.connection(), part_id)
            .await
            .map_err(project_outline_record_error_to_app_error)
    }

    pub async fn outline_generation_progress_rows(
        &self,
        project_id: i64,
    ) -> Result<OutlineGenerationProgressRows, AppError> {
        a4n_db::project::outline_generation_progress_rows(self.db.connection(), project_id)
            .await
            .map_err(project_outline_record_error_to_app_error)
    }
}
