mod data;
mod error;
mod persistence;
mod rules;

pub(crate) use persistence::create_source_snapshot_in_record;
pub(super) use rules::normalize_chapter_scope;
