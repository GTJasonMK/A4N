mod contracts;
mod execution;
mod persistence;
mod preparation;

pub use contracts::{
    SourceSyncReplacementSegmentRecordInput, SourceSyncReplacementSpecRecordInput,
};
pub use execution::create_replacement_generation_spec_record;
pub(in crate::source_sync) use execution::create_source_sync_replacement_spec_record;
