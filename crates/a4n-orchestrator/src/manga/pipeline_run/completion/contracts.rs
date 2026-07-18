use serde_json::Value;

use super::super::super::pipeline_persist::PageMangaPromptGenerationTrace;

pub(super) struct MangaCompletionArtifacts {
    pub(super) plan_json: Value,
    pub(super) dp_json: Value,
    pub(super) page_traces: Vec<PageMangaPromptGenerationTrace>,
}
