use a4n_core::{error::AppError, models::chapter::chapter_manga_prompt};

use super::record_errors::manga_prompt_record_error;
use super::{MangaPipelineStatusRows, MangaPromptUpdatedAt};
use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn manga_page_prompts_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_prompt_type: &str,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        a4n_db::manga::manga_page_prompts_by_chapter(
            self.db.connection(),
            project_id,
            chapter_id,
            page_prompt_type,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    pub async fn delete_manga_prompts_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<u64, AppError> {
        a4n_db::manga::delete_manga_prompts_by_chapter(self.db.connection(), project_id, chapter_id)
            .await
            .map_err(manga_prompt_record_error)
    }

    pub async fn delete_manga_prompt_record(&self, prompt_id: i64) -> Result<(), AppError> {
        a4n_db::manga::delete_manga_prompt_record(self.db.connection(), prompt_id)
            .await
            .map_err(manga_prompt_record_error)
    }

    pub async fn manga_pipeline_status_rows(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_prompt_type: &str,
        pipeline_status_prompt_type: &str,
    ) -> Result<MangaPipelineStatusRows, AppError> {
        a4n_db::manga::manga_pipeline_status_rows(
            self.db.connection(),
            project_id,
            chapter_id,
            page_prompt_type,
            pipeline_status_prompt_type,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    pub async fn manga_page_prompt_for_image_generation(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_number: i32,
        page_prompt_type: &str,
    ) -> Result<Option<chapter_manga_prompt::Model>, AppError> {
        a4n_db::manga::manga_page_prompt_for_image_generation(
            self.db.connection(),
            project_id,
            chapter_id,
            page_number,
            page_prompt_type,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    pub async fn mark_manga_page_prompt_image_completed_with_image(
        &self,
        page_prompt_id: i64,
        image_id: i64,
        updated_at: MangaPromptUpdatedAt,
    ) -> Result<(), AppError> {
        a4n_db::manga::mark_manga_page_prompt_image_completed_with_image(
            self.db.connection(),
            page_prompt_id,
            image_id,
            updated_at,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    pub async fn manga_page_prompt_by_id(
        &self,
        prompt_id: i64,
    ) -> Result<Option<chapter_manga_prompt::Model>, AppError> {
        a4n_db::manga::manga_page_prompt_by_id(self.db.connection(), prompt_id)
            .await
            .map_err(manga_prompt_record_error)
    }

    pub async fn update_manga_page_prompt_layout(
        &self,
        panel: chapter_manga_prompt::Model,
        layout: serde_json::Value,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        a4n_db::manga::update_manga_page_prompt_layout(self.db.connection(), panel, layout)
            .await
            .map_err(manga_prompt_record_error)
    }

    pub async fn update_manga_page_prompt_image_prompt(
        &self,
        panel: chapter_manga_prompt::Model,
        image_prompt: String,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        a4n_db::manga::update_manga_page_prompt_image_prompt(
            self.db.connection(),
            panel,
            image_prompt,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    pub async fn manga_reference_bound_asset(
        &self,
        project_id: i64,
        asset_kind: &str,
        asset_id: i64,
    ) -> Result<serde_json::Value, AppError> {
        a4n_db::manga::manga_reference_bound_asset(
            self.db.connection(),
            project_id,
            asset_kind,
            asset_id,
        )
        .await
        .map_err(manga_prompt_record_error)
    }
}
