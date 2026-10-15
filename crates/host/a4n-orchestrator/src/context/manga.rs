//! Manga 领域私有 owner。
//!
//! 持有 `DbHandle`，封装漫画页面提示词账本与输出同步的数据库行为。
//! 生成编排（LLM/preview/pipeline）仍在 `manga/` 模块，但 DB 只经本 owner。

use a4n_db::{
    DbHandle,
    manga::{
        MangaPipelineArtifactProjectionReset, MangaPipelineArtifactProjectionUpdate,
        MangaPromptUpdatedAt,
    },
};
use a4n_domain::manga::{
    MangaBoundReferenceAsset, MangaPagePromptRecord, MangaPipelineArtifactDraft,
    MangaPipelineArtifactRecord, MangaPipelineArtifactStage, MangaReferenceAssetKind,
};
use a4n_error::AppError;
use serde_json::Value;
use std::collections::HashMap;

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

    async fn delete_manga_prompts_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<u64, AppError> {
        a4n_db::manga::delete_manga_prompts_by_chapter(
            self.database.executor(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn delete_manga_page_by_number(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_number: i32,
    ) -> Result<(), AppError> {
        a4n_db::manga::delete_manga_page_by_number(
            self.database.executor(),
            project_id,
            chapter_id,
            page_number,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn manga_page_prompt_for_image_generation(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_number: i32,
    ) -> Result<Option<MangaPagePromptRecord>, AppError> {
        a4n_db::manga::manga_page_prompt_for_image_generation(
            self.database.executor(),
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
            self.database.executor(),
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
    ) -> Result<Option<MangaPagePromptRecord>, AppError> {
        a4n_db::manga::manga_page_prompt_by_id(self.database.executor(), prompt_id)
            .await
            .map_err(manga_prompt_record_error)
    }

    async fn manga_page_prompts_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<MangaPagePromptRecord>, AppError> {
        a4n_db::manga::manga_page_prompts_by_chapter(
            self.database.executor(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn manga_pipeline_artifacts_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<MangaPipelineArtifactRecord>, AppError> {
        a4n_db::manga::manga_pipeline_artifacts_by_chapter(
            self.database.executor(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn manga_pipeline_artifacts_by_chapters(
        &self,
        project_id: i64,
        chapter_ids: &[i64],
        stage: MangaPipelineArtifactStage,
    ) -> Result<HashMap<i64, MangaPipelineArtifactRecord>, AppError> {
        a4n_db::manga::manga_pipeline_artifacts_by_chapters(
            self.database.executor(),
            project_id,
            chapter_ids,
            stage,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn manga_pipeline_artifact(
        &self,
        project_id: i64,
        chapter_id: i64,
        stage: MangaPipelineArtifactStage,
        page_number: Option<u32>,
    ) -> Result<Option<MangaPipelineArtifactRecord>, AppError> {
        a4n_db::manga::manga_pipeline_artifact(
            self.database.executor(),
            project_id,
            chapter_id,
            stage,
            page_number,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn manga_pipeline_artifact_deletion_plan(
        &self,
        project_id: i64,
        chapter_id: i64,
        stage: MangaPipelineArtifactStage,
        page_number: Option<u32>,
    ) -> Result<Vec<MangaPipelineArtifactRecord>, AppError> {
        a4n_db::manga::manga_pipeline_artifact_deletion_plan(
            self.database.executor(),
            project_id,
            chapter_id,
            stage,
            page_number,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn upsert_manga_pipeline_artifact(
        &self,
        draft: MangaPipelineArtifactDraft,
    ) -> Result<MangaPipelineArtifactRecord, AppError> {
        a4n_db::manga::upsert_manga_pipeline_artifact(
            self.database.executor(),
            draft,
            chrono::Utc::now().naive_utc(),
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn save_manga_pipeline_artifact_with_projection(
        &self,
        draft: MangaPipelineArtifactDraft,
        projection: Option<MangaPipelineArtifactProjectionUpdate>,
    ) -> Result<MangaPipelineArtifactRecord, AppError> {
        a4n_db::manga::save_manga_pipeline_artifact_with_projection(
            self.database.executor(),
            draft,
            projection,
            chrono::Utc::now().naive_utc(),
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn delete_manga_pipeline_artifact(
        &self,
        project_id: i64,
        chapter_id: i64,
        stage: MangaPipelineArtifactStage,
        page_number: Option<u32>,
        expected_artifact_ids: &[i64],
        projection_resets: &[MangaPipelineArtifactProjectionReset],
    ) -> Result<Vec<MangaPipelineArtifactRecord>, AppError> {
        a4n_db::manga::delete_manga_pipeline_artifact(
            self.database.executor(),
            project_id,
            chapter_id,
            stage,
            page_number,
            expected_artifact_ids,
            projection_resets,
        )
        .await
        .map_err(manga_prompt_record_error)
    }

    async fn update_manga_page_prompt_layout(
        &self,
        page_prompt: MangaPagePromptRecord,
        layout: Value,
    ) -> Result<MangaPagePromptRecord, AppError> {
        a4n_db::manga::update_manga_page_prompt_layout(
            self.database.executor(),
            page_prompt,
            layout,
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
            self.database.executor(),
            project_id,
            asset_kind,
            asset_id,
        )
        .await
        .map_err(manga_prompt_record_error)
    }
}

impl OrchestratorContext {
    pub(crate) async fn delete_manga_prompts_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<u64, AppError> {
        self.manga
            .delete_manga_prompts_by_chapter(project_id, chapter_id)
            .await
    }

    pub(crate) async fn delete_manga_page_by_number(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_number: i32,
    ) -> Result<(), AppError> {
        self.manga
            .delete_manga_page_by_number(project_id, chapter_id, page_number)
            .await
    }

    pub(crate) async fn manga_page_prompt_for_image_generation(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_number: i32,
    ) -> Result<Option<MangaPagePromptRecord>, AppError> {
        self.manga
            .manga_page_prompt_for_image_generation(project_id, chapter_id, page_number)
            .await
    }

    pub(crate) async fn mark_manga_page_prompt_image_completed_with_image(
        &self,
        page_prompt_id: i64,
        image_id: i64,
        updated_at: MangaPromptUpdatedAt,
    ) -> Result<(), AppError> {
        self.manga
            .mark_manga_page_prompt_image_completed_with_image(page_prompt_id, image_id, updated_at)
            .await
    }

    pub(crate) async fn manga_page_prompt_by_id(
        &self,
        prompt_id: i64,
    ) -> Result<Option<MangaPagePromptRecord>, AppError> {
        self.manga.manga_page_prompt_by_id(prompt_id).await
    }

    pub(crate) async fn manga_page_prompts_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<MangaPagePromptRecord>, AppError> {
        self.manga
            .manga_page_prompts_by_chapter(project_id, chapter_id)
            .await
    }

    pub(crate) async fn manga_pipeline_artifacts_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<MangaPipelineArtifactRecord>, AppError> {
        self.manga
            .manga_pipeline_artifacts_by_chapter(project_id, chapter_id)
            .await
    }

    pub(crate) async fn manga_pipeline_artifacts_by_chapters(
        &self,
        project_id: i64,
        chapter_ids: &[i64],
        stage: MangaPipelineArtifactStage,
    ) -> Result<HashMap<i64, MangaPipelineArtifactRecord>, AppError> {
        self.manga
            .manga_pipeline_artifacts_by_chapters(project_id, chapter_ids, stage)
            .await
    }

    pub(crate) async fn manga_pipeline_artifact(
        &self,
        project_id: i64,
        chapter_id: i64,
        stage: MangaPipelineArtifactStage,
        page_number: Option<u32>,
    ) -> Result<Option<MangaPipelineArtifactRecord>, AppError> {
        self.manga
            .manga_pipeline_artifact(project_id, chapter_id, stage, page_number)
            .await
    }

    pub(crate) async fn manga_pipeline_artifact_deletion_plan(
        &self,
        project_id: i64,
        chapter_id: i64,
        stage: MangaPipelineArtifactStage,
        page_number: Option<u32>,
    ) -> Result<Vec<MangaPipelineArtifactRecord>, AppError> {
        self.manga
            .manga_pipeline_artifact_deletion_plan(project_id, chapter_id, stage, page_number)
            .await
    }

    pub(crate) async fn upsert_manga_pipeline_artifact(
        &self,
        draft: MangaPipelineArtifactDraft,
    ) -> Result<MangaPipelineArtifactRecord, AppError> {
        self.manga.upsert_manga_pipeline_artifact(draft).await
    }

    pub(crate) async fn save_manga_pipeline_artifact_with_projection(
        &self,
        draft: MangaPipelineArtifactDraft,
        projection: Option<MangaPipelineArtifactProjectionUpdate>,
    ) -> Result<MangaPipelineArtifactRecord, AppError> {
        self.manga
            .save_manga_pipeline_artifact_with_projection(draft, projection)
            .await
    }

    pub(crate) async fn delete_manga_pipeline_artifact(
        &self,
        project_id: i64,
        chapter_id: i64,
        stage: MangaPipelineArtifactStage,
        page_number: Option<u32>,
        expected_artifact_ids: &[i64],
        projection_resets: &[MangaPipelineArtifactProjectionReset],
    ) -> Result<Vec<MangaPipelineArtifactRecord>, AppError> {
        self.manga
            .delete_manga_pipeline_artifact(
                project_id,
                chapter_id,
                stage,
                page_number,
                expected_artifact_ids,
                projection_resets,
            )
            .await
    }

    pub(crate) async fn update_manga_page_prompt_layout(
        &self,
        page_prompt: MangaPagePromptRecord,
        layout: Value,
    ) -> Result<MangaPagePromptRecord, AppError> {
        self.manga
            .update_manga_page_prompt_layout(page_prompt, layout)
            .await
    }

    pub(crate) async fn manga_reference_bound_asset(
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
