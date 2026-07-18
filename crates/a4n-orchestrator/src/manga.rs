//! 漫画导演计划驱动的七阶段生成流水线与漫画提示词账本。

use std::collections::HashMap;

use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

use crate::OrchestratorContext;

mod checkpoint;
mod output_sync;
mod pipeline_json;
mod pipeline_persist;
mod pipeline_preview;
mod pipeline_prompt;
mod pipeline_run;
mod pipeline_state;
mod pipeline_status;
mod record_errors;
mod records;
mod reference_assets;
mod shooting_script_projection;
mod source;
mod workflow;

pub use pipeline_preview::MangaPipelineRequestPreviewStep;
pub use pipeline_state::MangaPipelineStartStage;
pub use reference_assets::{MangaCharacterPortraitRef, MangaReferenceContext};
pub use source::{ensure_manga_source_can_write, manga_checkpoint_matches_source};
pub use workflow::{
    manga_pipeline_business_key, manga_pipeline_fingerprint, parse_manga_pipeline_start_stage,
};

pub const PIPELINE_STATUS_PROMPT_TYPE: &str = "pipeline_status";
pub const MANGA_PAGE_PROMPT_TYPE: &str = "page_generation";
pub const MANGA_PANEL_PLAN_TYPE: &str = "panel_plan";
pub const PIPELINE_TOTAL_STEPS: i32 = 7;

pub struct MangaGeneratePanelsInput<'a> {
    pub chapter_id: i64,
    pub project_id: i64,
    pub prompt_renderer: &'a dyn PromptRenderer,
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
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub reference_context: MangaReferenceContext,
    pub workflow_run_id: Option<i64>,
    pub resume_from_checkpoint: bool,
    pub start_from_stage: Option<MangaPipelineStartStage>,
}

#[cfg(test)]
mod tests;

pub use a4n_db::manga::{MangaPipelineStatusRows, MangaPromptRecordError, MangaPromptUpdatedAt};
