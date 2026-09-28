use super::super::super::persist::PageMangaPromptGenerationTrace;
use a4n_domain::manga::{MangaPageDrawingPromptV2, MangaPagePlanV4};

pub(super) struct MangaCompletionArtifacts {
    pub(super) plan: MangaPagePlanV4,
    pub(super) page_plan_ref: a4n_domain::media_authoring::AuthoringDocumentRef,
    pub(super) page_prompts: Vec<MangaPageDrawingPromptV2>,
    pub(super) page_traces: Vec<PageMangaPromptGenerationTrace>,
}
