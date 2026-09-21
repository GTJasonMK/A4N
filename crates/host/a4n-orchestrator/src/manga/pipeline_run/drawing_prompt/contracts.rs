use std::collections::HashMap;

use serde_json::Value;

use a4n_domain::manga::{MangaPageDrawingPromptV2, MangaPagePlanV3, MangaPageV3};
use a4n_prompt::rendering::PromptRenderer;

use super::super::super::pipeline_persist::PageMangaPromptGenerationTrace;
use super::super::super::pipeline_prompt::MangaPromptGenerationTrace;

pub(in crate::manga::pipeline_run) struct DrawingPromptStepInput<'a> {
    pub(in crate::manga::pipeline_run) chapter_id: i64,
    pub(in crate::manga::pipeline_run) project_id: i64,
    pub(in crate::manga::pipeline_run) prompt_renderer: &'a dyn PromptRenderer,
    pub(in crate::manga::pipeline_run) vars: &'a HashMap<String, Value>,
    pub(in crate::manga::pipeline_run) task_id: &'a str,
    pub(in crate::manga::pipeline_run) workflow_run_id: i64,
    pub(in crate::manga::pipeline_run) reference_assets: &'a Value,
    pub(in crate::manga::pipeline_run) plan: &'a MangaPagePlanV3,
    pub(in crate::manga::pipeline_run) extraction_summary: &'a Value,
    pub(in crate::manga::pipeline_run) restore_from_checkpoint: bool,
}

pub(in crate::manga::pipeline_run) struct DrawingPromptStepOutput {
    pub(in crate::manga::pipeline_run) page_prompts: Vec<MangaPageDrawingPromptV2>,
    pub(in crate::manga::pipeline_run) page_traces: Vec<PageMangaPromptGenerationTrace>,
}

pub(super) struct SinglePageDrawingPromptInput<'a> {
    pub(super) chapter_id: i64,
    pub(super) project_id: i64,
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) vars: &'a HashMap<String, Value>,
    pub(super) task_id: &'a str,
    pub(super) workflow_run_id: i64,
    pub(super) reference_assets: &'a Value,
    pub(super) page_plan: &'a MangaPageV3,
    pub(super) page_index: usize,
    pub(super) total_pages: usize,
    pub(super) extraction_summary: &'a Value,
    pub(super) restore_from_checkpoint: bool,
}

pub(super) struct SinglePageDrawingPromptOutput {
    pub(super) prompt: MangaPageDrawingPromptV2,
    pub(super) trace: MangaPromptGenerationTrace,
}
