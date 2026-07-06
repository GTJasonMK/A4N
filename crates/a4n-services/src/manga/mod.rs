//! 漫画生成服务：漫画整页提示词生成
//!
//! 对应文档 10-漫画生成系统；完整 8 步流水线：
//!   1. manga_event_beats：抽取漫画化事件卡片
//!   2. manga_character_binding：绑定出场角色身份和外观
//!   3. manga_speech_selection：选择关键对白、想法与旁白
//!   4. manga_visual_anchor_extraction：提取场景和视觉锚点
//!   5. manga_item_extraction：抽取关键物品
//!   6. manga_chapter_comic_synthesis：综合章节节奏
//!   7. manga_page_planning：分页规划
//!   8. manga_page_drawing_prompt：按页编排可直接生成整页漫画的最终 prompt 并写入

use sea_orm::*;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_manga_prompt;
use a4n_engine::llm::provider::LlmProvider;
use a4n_engine::prompt::engine::PromptEngine;

use crate::generation::delete_generation_run_targets_for;
use crate::source::{
    ChapterSourceContext, source_content_sha256, source_metadata, with_source_metadata,
};

mod checkpoint;
mod output_sync;
mod pipeline_json;
mod pipeline_persist;
mod pipeline_preview;
mod pipeline_prompt;
mod pipeline_run;
mod pipeline_state;
mod pipeline_status;
mod reference_assets;

pub use self::pipeline_preview::MangaPipelineRequestPreviewStep;
pub use self::pipeline_state::MangaPipelineStartStage;
pub use self::reference_assets::{MangaCharacterPortraitRef, MangaReferenceContext};

pub const PIPELINE_STATUS_PROMPT_TYPE: &str = "pipeline_status";
pub const MANGA_EXTRACTION_SUMMARY_TYPE: &str = "manga_extraction_summary";
pub const MANGA_PAGE_PROMPT_TYPE: &str = "page_generation";
pub const MANGA_PAGE_PROMPT_RAW_TYPE: &str = "page_drawing_prompt";
pub const PIPELINE_TOTAL_STEPS: i32 = 8;

#[derive(Debug, Clone, Copy)]
pub struct ChapterMangaSourceContext<'a> {
    pub content: &'a str,
}

pub struct MangaGeneratePanelsInput<'a> {
    pub chapter_id: i64,
    pub project_id: i64,
    pub source: ChapterMangaSourceContext<'a>,
    pub llm: &'a dyn LlmProvider,
    pub prompt_engine: &'a dyn PromptEngine,
    pub variables: &'a HashMap<String, Value>,
    pub reference_context: MangaReferenceContext,
    pub task_id: Option<&'a str>,
    pub workflow_run_id: Option<i64>,
    pub resume_from_checkpoint: bool,
    pub start_from_stage: Option<MangaPipelineStartStage>,
}

pub struct MangaPipelineRequestPreviewInput<'a> {
    pub chapter_id: i64,
    pub project_id: i64,
    pub source: ChapterMangaSourceContext<'a>,
    pub prompt_engine: &'a dyn PromptEngine,
    pub variables: &'a HashMap<String, Value>,
    pub reference_context: MangaReferenceContext,
    pub workflow_run_id: Option<i64>,
    pub resume_from_checkpoint: bool,
    pub start_from_stage: Option<MangaPipelineStartStage>,
}

#[derive(Clone)]
pub struct MangaService {
    db: Arc<DatabaseConnection>,
}

fn manga_source_content_sha256(content: &str) -> String {
    source_content_sha256(content)
}

fn manga_source_metadata(source: ChapterMangaSourceContext<'_>) -> Value {
    source_metadata(ChapterSourceContext {
        content: source.content,
    })
}

fn with_manga_source_metadata(
    layout: Option<Value>,
    source: ChapterMangaSourceContext<'_>,
) -> Option<Value> {
    with_source_metadata(
        layout,
        ChapterSourceContext {
            content: source.content,
        },
    )
}

impl MangaService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 列出章节所有漫画整页提示词
    pub async fn list_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        chapter_manga_prompt::Entity::find()
            .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
            .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
            .filter(chapter_manga_prompt::Column::PromptType.eq(MANGA_PAGE_PROMPT_TYPE))
            .order_by_asc(chapter_manga_prompt::Column::PageNumber)
            .order_by_asc(chapter_manga_prompt::Column::PanelNumber)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    /// 删除章节所有漫画产物
    pub async fn delete_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let prompt_ids = chapter_manga_prompt::Entity::find()
            .select_only()
            .column(chapter_manga_prompt::Column::Id)
            .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
            .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
            .into_tuple::<i64>()
            .all(&txn)
            .await
            .map_err(AppError::Database)?;
        delete_generation_run_targets_for(&txn, "chapter_manga_prompt", &prompt_ids).await?;
        chapter_manga_prompt::Entity::delete_many()
            .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
            .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
            .exec(&txn)
            .await
            .map_err(AppError::Database)?;
        txn.commit().await.map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn delete_prompt(&self, prompt_id: i64) -> Result<(), AppError> {
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        chapter_manga_prompt::Entity::find_by_id(prompt_id)
            .one(&txn)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("漫画提示词不存在".into()))?;
        delete_generation_run_targets_for(&txn, "chapter_manga_prompt", &[prompt_id]).await?;
        chapter_manga_prompt::Entity::delete_by_id(prompt_id)
            .exec(&txn)
            .await
            .map_err(AppError::Database)?;
        txn.commit().await.map_err(AppError::Database)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
