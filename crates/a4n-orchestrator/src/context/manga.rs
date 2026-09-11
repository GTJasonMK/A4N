//! Manga 领域私有 owner。
//!
//! 持有 `DbHandle`，封装漫画页面提示词账本与输出同步的数据库行为。
//! 生成编排（LLM/preview/pipeline）仍在 `manga/` 模块，但 DB 只经本 owner。

use a4n_db::{DbHandle, manga::MangaPromptUpdatedAt};
use a4n_db_models::models::chapter::chapter_manga_prompt;
use a4n_domain::manga::{MangaBoundReferenceAsset, MangaReferenceAssetKind};
use a4n_error::AppError;
use serde_json::Value;

use super::OrchestratorContext;
use crate::manga::manga_prompt_record_error;

/// Manga 领域 application owner。
#[derive(Clone)]
pub(super) struct MangaApplication {
    database: DbHandle,
}

impl MangaApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn manga_page_prompts_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        a4n_db::manga::manga_page_prompts_by_chapter(
            self.database.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn manga_page_prompt_statuses_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<(i64, String)>, AppError> {
        a4n_db::manga::manga_page_prompt_statuses_by_chapter(
            self.database.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn manga_page_prompt_statuses_by_chapters(
        &self,
        project_id: i64,
        chapter_ids: &[i64],
    ) -> Result<Vec<(i64, i64, String)>, AppError> {
        a4n_db::manga::manga_page_prompt_statuses_by_chapters(
            self.database.connection(),
            project_id,
            chapter_ids,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn delete_manga_prompts_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<u64, AppError> {
        a4n_db::manga::delete_manga_prompts_by_chapter(
            self.database.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn delete_manga_prompt_record(&self, prompt_id: i64) -> Result<(), AppError> {
        a4n_db::manga::delete_manga_prompt_record(self.database.connection(), prompt_id)
            .await
            .map_err(manga_prompt_record_error)
    }

    async fn manga_page_prompt_for_image_generation(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_number: i32,
    ) -> Result<Option<chapter_manga_prompt::Model>, AppError> {
        a4n_db::manga::manga_page_prompt_for_image_generation(
            self.database.connection(),
            project_id,
            chapter_id,
            page_number,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn mark_manga_page_prompt_image_completed_with_image(
        &self,
        page_prompt_id: i64,
        image_id: i64,
        updated_at: MangaPromptUpdatedAt,
    ) -> Result<(), AppError> {
        a4n_db::manga::mark_manga_page_prompt_image_completed_with_image(
            self.database.connection(),
            page_prompt_id,
            image_id,
            updated_at,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn manga_page_prompt_by_id(
        &self,
        prompt_id: i64,
    ) -> Result<Option<chapter_manga_prompt::Model>, AppError> {
        a4n_db::manga::manga_page_prompt_by_id(self.database.connection(), prompt_id)
            .await
            .map_err(manga_prompt_record_error)
    }

    async fn update_manga_page_prompt_layout(
        &self,
        page_prompt: chapter_manga_prompt::Model,
        layout: Value,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        a4n_db::manga::update_manga_page_prompt_layout(
            self.database.connection(),
            page_prompt,
            layout,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn update_manga_page_prompt_image_prompt(
        &self,
        page_prompt: chapter_manga_prompt::Model,
        image_prompt: String,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        a4n_db::manga::update_manga_page_prompt_image_prompt(
            self.database.connection(),
            page_prompt,
            image_prompt,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn manga_reference_bound_asset(
        &self,
        project_id: i64,
        asset_kind: MangaReferenceAssetKind,
        asset_id: i64,
    ) -> Result<MangaBoundReferenceAsset, AppError> {
        a4n_db::manga::manga_reference_bound_asset(
            self.database.connection(),
            project_id,
            asset_kind,
            asset_id,
        )
        .await
        .map_err(manga_prompt_record_error)
    }
}

impl OrchestratorContext {
    pub async fn manga_page_prompts_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        self.manga
            .manga_page_prompts_by_chapter(project_id, chapter_id)
            .await
    }

    pub async fn manga_page_prompt_statuses_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<(i64, String)>, AppError> {
        self.manga
            .manga_page_prompt_statuses_by_chapter(project_id, chapter_id)
            .await
    }

    pub(crate) async fn manga_page_prompt_statuses_by_chapters(
        &self,
        project_id: i64,
        chapter_ids: &[i64],
    ) -> Result<Vec<(i64, i64, String)>, AppError> {
        self.manga
            .manga_page_prompt_statuses_by_chapters(project_id, chapter_ids)
            .await
    }

    pub async fn delete_manga_prompts_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<u64, AppError> {
        self.manga
            .delete_manga_prompts_by_chapter(project_id, chapter_id)
            .await
    }

    pub async fn delete_manga_prompt_record(&self, prompt_id: i64) -> Result<(), AppError> {
        self.manga.delete_manga_prompt_record(prompt_id).await
    }

    pub async fn manga_page_prompt_for_image_generation(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_number: i32,
    ) -> Result<Option<chapter_manga_prompt::Model>, AppError> {
        self.manga
            .manga_page_prompt_for_image_generation(project_id, chapter_id, page_number)
            .await
    }

    pub async fn mark_manga_page_prompt_image_completed_with_image(
        &self,
        page_prompt_id: i64,
        image_id: i64,
        updated_at: MangaPromptUpdatedAt,
    ) -> Result<(), AppError> {
        self.manga
            .mark_manga_page_prompt_image_completed_with_image(page_prompt_id, image_id, updated_at)
            .await
    }

    pub async fn manga_page_prompt_by_id(
        &self,
        prompt_id: i64,
    ) -> Result<Option<chapter_manga_prompt::Model>, AppError> {
        self.manga.manga_page_prompt_by_id(prompt_id).await
    }

    pub async fn update_manga_page_prompt_layout(
        &self,
        page_prompt: chapter_manga_prompt::Model,
        layout: Value,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        self.manga
            .update_manga_page_prompt_layout(page_prompt, layout)
            .await
    }

    pub async fn update_manga_page_prompt_image_prompt(
        &self,
        page_prompt: chapter_manga_prompt::Model,
        image_prompt: String,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        self.manga
            .update_manga_page_prompt_image_prompt(page_prompt, image_prompt)
            .await
    }

    pub async fn manga_reference_bound_asset(
        &self,
        project_id: i64,
        asset_kind: MangaReferenceAssetKind,
        asset_id: i64,
    ) -> Result<MangaBoundReferenceAsset, AppError> {
        self.manga
            .manga_reference_bound_asset(project_id, asset_kind, asset_id)
            .await
    }
}
