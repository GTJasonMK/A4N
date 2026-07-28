mod chapters;
mod manifest;
mod outlines_profiles;
mod rules;

pub(in crate::project::family_package::validation) use manifest::validate_source_snapshot_manifest;
pub(in crate::project::family_package::validation) use rules::parse_chapter_scope_json;
