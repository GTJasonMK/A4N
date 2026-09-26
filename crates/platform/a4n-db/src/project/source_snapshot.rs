mod creation;
mod derived;
mod hashing;
mod outlines;
mod profiles;
mod queries;
mod types;

pub(crate) use creation::{create_source_snapshot_in_record, source_snapshot_version_hash};
pub use derived::create_derived_project_record;
pub(crate) use hashing::shooting_script_source_hash;
pub use profiles::scene_profiles_for_source;
pub(crate) use queries::projects_using_source_snapshot;
pub use queries::{
    source_snapshot_chapter_content_by_snapshot_and_source, source_snapshot_chapters,
};
pub use types::{CreateDerivedProjectParams, DerivedChapterScope};

#[cfg(test)]
mod tests;
