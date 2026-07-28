//! Manga 领域私有 owner。
//!
//! 持有 `DbHandle`，封装漫画提示词账本、流水线状态/断点与输出同步的数据库行为。
//! 生成编排（LLM/preview/pipeline）仍在 `manga/` 模块，但 DB 只经本 owner。

use a4n_core::{error::AppError, models::chapter::chapter_manga_prompt};
use a4n_db::DbHandle;
use a4n_db::manga::{SaveMangaPipelineCheckpointInput, SetMangaPipelineStatusInput};
use serde_json::Value;

use super::OrchestratorContext;
use crate::manga::record_errors::{manga_pipeline_db_error, manga_prompt_record_error};
use crate::manga::{
    MangaPipelineCheckpointInput, MangaPipelineStatusInput, MangaPipelineStatusRows,
    MangaPromptUpdatedAt,
};

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
        page_prompt_type: &str,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        a4n_db::manga::manga_page_prompts_by_chapter(
            self.database.connection(),
            project_id,
            chapter_id,
            page_prompt_type,
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

    async fn manga_pipeline_status_rows(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_prompt_type: &str,
        pipeline_status_prompt_type: &str,
    ) -> Result<MangaPipelineStatusRows, AppError> {
        a4n_db::manga::manga_pipeline_status_rows(
            self.database.connection(),
            project_id,
            chapter_id,
            page_prompt_type,
            pipeline_status_prompt_type,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn manga_page_prompt_for_image_generation(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_number: i32,
        page_prompt_type: &str,
    ) -> Result<Option<chapter_manga_prompt::Model>, AppError> {
        a4n_db::manga::manga_page_prompt_for_image_generation(
            self.database.connection(),
            project_id,
            chapter_id,
            page_number,
            page_prompt_type,
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
        panel: chapter_manga_prompt::Model,
        layout: Value,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        a4n_db::manga::update_manga_page_prompt_layout(self.database.connection(), panel, layout)
            .await
            .map_err(manga_prompt_record_error)
    }

    async fn update_manga_page_prompt_image_prompt(
        &self,
        panel: chapter_manga_prompt::Model,
        image_prompt: String,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        a4n_db::manga::update_manga_page_prompt_image_prompt(
            self.database.connection(),
            panel,
            image_prompt,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn manga_reference_bound_asset(
        &self,
        project_id: i64,
        asset_kind: &str,
        asset_id: i64,
    ) -> Result<Value, AppError> {
        a4n_db::manga::manga_reference_bound_asset(
            self.database.connection(),
            project_id,
            asset_kind,
            asset_id,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn set_pipeline_status(
        &self,
        input: SetMangaPipelineStatusInput,
    ) -> Result<(), AppError> {
        a4n_db::manga::set_manga_pipeline_status(self.database.connection(), input)
            .await
            .map_err(manga_pipeline_db_error)
    }

    async fn pipeline_status_row(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Option<chapter_manga_prompt::Model>, AppError> {
        a4n_db::manga::manga_pipeline_status_row(self.database.connection(), project_id, chapter_id)
            .await
            .map_err(manga_pipeline_db_error)
    }

    async fn clear_pipeline_checkpoint(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::manga::clear_manga_pipeline_checkpoint(
            self.database.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(manga_pipeline_db_error)
    }

    async fn save_pipeline_checkpoint(
        &self,
        input: SaveMangaPipelineCheckpointInput,
    ) -> Result<(), AppError> {
        a4n_db::manga::save_manga_pipeline_checkpoint(self.database.connection(), input)
            .await
            .map_err(manga_pipeline_db_error)
    }
}

impl OrchestratorContext {
    pub async fn manga_page_prompts_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_prompt_type: &str,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        self.manga
            .manga_page_prompts_by_chapter(project_id, chapter_id, page_prompt_type)
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

    pub async fn manga_pipeline_status_rows(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_prompt_type: &str,
        pipeline_status_prompt_type: &str,
    ) -> Result<MangaPipelineStatusRows, AppError> {
        self.manga
            .manga_pipeline_status_rows(
                project_id,
                chapter_id,
                page_prompt_type,
                pipeline_status_prompt_type,
            )
            .await
    }

    pub async fn manga_page_prompt_for_image_generation(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_number: i32,
        page_prompt_type: &str,
    ) -> Result<Option<chapter_manga_prompt::Model>, AppError> {
        self.manga
            .manga_page_prompt_for_image_generation(
                project_id,
                chapter_id,
                page_number,
                page_prompt_type,
            )
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
        panel: chapter_manga_prompt::Model,
        layout: Value,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        self.manga
            .update_manga_page_prompt_layout(panel, layout)
            .await
    }

    pub async fn update_manga_page_prompt_image_prompt(
        &self,
        panel: chapter_manga_prompt::Model,
        image_prompt: String,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        self.manga
            .update_manga_page_prompt_image_prompt(panel, image_prompt)
            .await
    }

    pub async fn manga_reference_bound_asset(
        &self,
        project_id: i64,
        asset_kind: &str,
        asset_id: i64,
    ) -> Result<Value, AppError> {
        self.manga
            .manga_reference_bound_asset(project_id, asset_kind, asset_id)
            .await
    }

    fn set_status_from_domain(input: MangaPipelineStatusInput<'_>) -> SetMangaPipelineStatusInput {
        SetMangaPipelineStatusInput {
            chapter_id: input.chapter_id,
            project_id: input.project_id,
            pipeline_status: input.pipeline_status.to_string(),
            stage: input.stage.to_string(),
            current: input.current,
            total: input.total,
            message: input.message.to_string(),
            task_id: input.task_id.map(str::to_string),
            allow_cancel_override: input.allow_cancel_override,
        }
    }

    /// 供 manga 子模块委托：接受领域层 `MangaPipelineStatusInput`。
    pub(crate) async fn set_pipeline_status(
        &self,
        input: MangaPipelineStatusInput<'_>,
    ) -> Result<(), AppError> {
        self.manga
            .set_pipeline_status(Self::set_status_from_domain(input))
            .await
    }

    pub(crate) async fn pipeline_status_row(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Option<chapter_manga_prompt::Model>, AppError> {
        self.manga.pipeline_status_row(project_id, chapter_id).await
    }

    pub(crate) async fn clear_pipeline_checkpoint(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        self.manga
            .clear_pipeline_checkpoint(project_id, chapter_id)
            .await
    }

    /// 供 manga 子模块委托：接受领域层 `MangaPipelineCheckpointInput`。
    pub(crate) async fn save_pipeline_checkpoint(
        &self,
        input: MangaPipelineCheckpointInput<'_>,
    ) -> Result<(), AppError> {
        let db_input = SaveMangaPipelineCheckpointInput {
            status: Self::set_status_from_domain(input.status),
            source_metadata: input.source_metadata.clone(),
            reference_assets_metadata: input.reference_assets_metadata.clone(),
            prompt_fingerprint: input.prompt_fingerprint.clone(),
            checkpoint_data: input.checkpoint_data.clone(),
        };
        self.manga.save_pipeline_checkpoint(db_input).await
    }
}
