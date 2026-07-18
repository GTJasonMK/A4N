use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::pipeline_json::PageDrawingPrompt;
use super::super::pipeline_prompt::MangaPromptGenerationTrace;

pub(in crate::manga) struct MangaPipelinePersistInput<'a> {
    pub(in crate::manga) chapter_id: i64,
    pub(in crate::manga) project_id: i64,
    pub(in crate::manga) source_metadata: &'a Value,
    pub(in crate::manga) reference_assets_metadata: &'a Value,
    pub(in crate::manga) prompt_fingerprint: &'a Value,
    pub(in crate::manga) plan_json: &'a Value,
    pub(in crate::manga) page_traces: Vec<PageMangaPromptGenerationTrace>,
    pub(in crate::manga) page_prompts: Vec<PageDrawingPrompt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(in crate::manga) struct PageMangaPromptGenerationTrace {
    pub(in crate::manga) page_number: i32,
    pub(in crate::manga) trace: MangaPromptGenerationTrace,
}
