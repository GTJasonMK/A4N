mod chapter_diff;
mod contracts;
mod diff;
mod profile_diff;
mod scope;
mod version;

pub use contracts::{
    SourceSnapshotChapterChangeRecord, SourceSnapshotDiffRecord, SourceSnapshotProfileChangeRecord,
};
pub use diff::diff_source_snapshots;
pub(in crate::source_sync) use scope::parse_source_sync_chapter_scope;
pub use scope::source_sync_chapter_scope_for_derived_project;
pub(in crate::source_sync) use version::calculate_source_sync_version_hash;

#[cfg(test)]
mod tests;
