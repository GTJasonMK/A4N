pub use a4n_api_contract::video::GeneratedVideoSummary;
pub use a4n_orchestrator::canvas::GenerateCanvasVideoInput;
pub use a4n_orchestrator::video::{
    GenerateChapterVideoInput, GenerateReferenceMediaVideoInput,
    PreviewChapterVideoPromptOverrideInput, ReferenceMediaVideoInput,
};

mod summary;
pub(crate) use summary::video_to_summary;
