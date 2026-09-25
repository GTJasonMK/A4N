mod cleanup;
pub use cleanup::cleanup_foreshadowing_for_chapter;

mod contracts;
pub use contracts::{
    ForeshadowingAnalysisDraft, ForeshadowingAnalysisSyncResult, ForeshadowingDbError,
};

mod records;
pub use records::{
    create_foreshadowing_record, delete_foreshadowing_record, foreshadowing_records_by_project,
    update_foreshadowing_record,
};

mod rules;

mod sync;
pub use sync::sync_chapter_analysis_foreshadowing;

#[cfg(test)]
mod tests;
