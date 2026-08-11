mod prelude {
    pub(super) use serde::{Deserialize, Serialize};
    pub(super) use tokio_util::sync::CancellationToken;
}

mod api;
mod contracts;
mod execution;
mod host_port;

pub use api::{
    generate_chapter_analysis_with_progress, preflight_generate_chapter_analysis,
    preflight_resume_chapter_analysis_sync, preview_chapter_analysis_request,
    preview_resume_chapter_analysis_sync_request, resume_chapter_analysis_sync_with_progress,
    resume_chapter_analysis_workflow_run,
};
pub use contracts::{
    CHAPTER_ANALYSIS_PROGRESS_TOTAL, ChapterAnalysisMutationResult, ChapterAnalysisProgress,
    GenerateChapterAnalysisInput,
};
pub use execution::generate_and_sync_chapter_analysis_with_progress;
pub(crate) use host_port::RuntimeChapterAnalysisHostPort;
