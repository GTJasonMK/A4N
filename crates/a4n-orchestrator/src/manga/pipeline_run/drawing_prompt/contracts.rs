use std::collections::HashMap;

use serde_json::{Map, Value};

use a4n_core::source_spans::SourceSpan;
use a4n_prompt::rendering::PromptRenderer;

use super::super::super::pipeline_persist::PageMangaPromptGenerationTrace;
use super::super::super::pipeline_prompt::MangaPromptGenerationTrace;

pub(in crate::manga::pipeline_run) struct DrawingPromptStepInput<'a> {
    pub(in crate::manga::pipeline_run) chapter_id: i64,
    pub(in crate::manga::pipeline_run) project_id: i64,
    pub(in crate::manga::pipeline_run) prompt_renderer: &'a dyn PromptRenderer,
    pub(in crate::manga::pipeline_run) vars: &'a HashMap<String, Value>,
    pub(in crate::manga::pipeline_run) task_id: Option<&'a str>,
    pub(in crate::manga::pipeline_run) workflow_run_id: Option<i64>,
    pub(in crate::manga::pipeline_run) source_metadata: &'a Value,
    pub(in crate::manga::pipeline_run) reference_assets_metadata: &'a Value,
    pub(in crate::manga::pipeline_run) prompt_fingerprint: &'a Value,
    pub(in crate::manga::pipeline_run) checkpoint: &'a mut Map<String, Value>,
    pub(in crate::manga::pipeline_run) chapter_content: &'a str,
    pub(in crate::manga::pipeline_run) source_spans: &'a [SourceSpan],
    pub(in crate::manga::pipeline_run) reference_assets: &'a Value,
    pub(in crate::manga::pipeline_run) plan_json: &'a Value,
    pub(in crate::manga::pipeline_run) extraction_summary: &'a Value,
    pub(in crate::manga::pipeline_run) restore_from_checkpoint: bool,
}

pub(in crate::manga::pipeline_run) struct DrawingPromptStepOutput {
    pub(in crate::manga::pipeline_run) json: Value,
    pub(in crate::manga::pipeline_run) page_traces: Vec<PageMangaPromptGenerationTrace>,
}

pub(super) struct SinglePageDrawingPromptInput<'a> {
    pub(super) chapter_id: i64,
    pub(super) project_id: i64,
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) vars: &'a HashMap<String, Value>,
    pub(super) task_id: Option<&'a str>,
    pub(super) workflow_run_id: Option<i64>,
    pub(super) source_metadata: &'a Value,
    pub(super) reference_assets_metadata: &'a Value,
    pub(super) prompt_fingerprint: &'a Value,
    pub(super) checkpoint: &'a mut Map<String, Value>,
    pub(super) chapter_content: &'a str,
    pub(super) source_spans: &'a [SourceSpan],
    pub(super) reference_assets: &'a Value,
    pub(super) page_plan: &'a Value,
    pub(super) page_index: usize,
    pub(super) total_pages: usize,
    pub(super) extraction_summary: &'a Value,
    pub(super) restore_from_checkpoint: bool,
}

pub(super) struct SinglePageDrawingPromptOutput {
    pub(super) page_number: i32,
    pub(super) prompt: Value,
    pub(super) trace: MangaPromptGenerationTrace,
}
