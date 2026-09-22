use super::super::super::persist::PageMangaPromptGenerationTrace;
use a4n_domain::manga::{MangaPageDrawingPromptV2, MangaPagePlanV3};

pub(super) struct MangaCompletionArtifacts {
    pub(super) plan: MangaPagePlanV3,
    pub(super) page_prompts: Vec<MangaPageDrawingPromptV2>,
    pub(super) page_traces: Vec<PageMangaPromptGenerationTrace>,
}
