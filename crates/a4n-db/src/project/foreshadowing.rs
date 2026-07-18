mod cleanup;
pub use cleanup::cleanup_foreshadowing_for_chapter;

mod contracts;
pub use contracts::{
    ForeshadowingAnalysisDraft, ForeshadowingAnalysisSyncResult, ForeshadowingCleanupResult,
    ForeshadowingDbError,
};

mod rules;

mod sync;
pub use sync::sync_chapter_analysis_foreshadowing;

#[cfg(test)]
mod tests;
