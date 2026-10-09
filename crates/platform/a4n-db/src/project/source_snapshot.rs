mod creation;
mod derived;
mod hashing;
mod outlines;
mod profiles;
mod queries;
mod types;

pub(crate) use creation::{create_source_snapshot_in_record, source_snapshot_version_hash};
pub use derived::create_derived_project_record;
pub use profiles::{authoring_scene_profiles, scene_profiles_for_source};
pub use queries::{
    source_snapshot_chapter_content_by_snapshot_and_source, source_snapshot_chapter_for_project,
    source_snapshot_chapter_index, source_snapshot_chapter_index_for_scope,
    source_snapshot_chapters,
};
pub use types::{CreateDerivedProjectParams, DerivedChapterScope};

#[cfg(test)]
mod tests;
