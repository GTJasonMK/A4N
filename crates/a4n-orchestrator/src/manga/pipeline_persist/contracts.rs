use a4n_domain::manga::{MangaPageDrawingPromptV2, MangaPagePlanV3};
use serde::Serialize;
use serde_json::Value;

use super::super::pipeline_prompt::MangaPromptGenerationTrace;

pub(in crate::manga) struct MangaPipelinePersistInput<'a> {
    pub(in crate::manga) chapter_id: i64,
    pub(in crate::manga) project_id: i64,
    pub(in crate::manga) source_metadata: &'a Value,
    pub(in crate::manga) reference_assets_metadata: &'a Value,
    pub(in crate::manga) prompt_fingerprint: &'a Value,
    pub(in crate::manga) plan: &'a MangaPagePlanV3,
    pub(in crate::manga) page_traces: Vec<PageMangaPromptGenerationTrace>,
    pub(in crate::manga) page_prompts: Vec<MangaPageDrawingPromptV2>,
}

#[derive(Debug, Clone, Serialize)]
pub(in crate::manga) struct PageMangaPromptGenerationTrace {
    pub(in crate::manga) page_number: u32,
    pub(in crate::manga) trace: MangaPromptGenerationTrace,
}
