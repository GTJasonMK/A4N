mod contracts;
pub(super) use contracts::foreshadowing_index_record;
pub use contracts::{
    ForeshadowingAnalysisDraft, ForeshadowingAnalysisSyncResult, ForeshadowingDbError,
    ForeshadowingFactUpdate,
};

mod records;
pub use records::{
    create_foreshadowing_read_record, create_foreshadowing_record, delete_foreshadowing_record,
    foreshadowing_index_records_by_project, foreshadowing_records_by_project,
    update_foreshadowing_read_record, update_foreshadowing_record,
};

mod sync;
pub(crate) use sync::restore_foreshadowing_fact;
pub use sync::sync_chapter_analysis_foreshadowing;
