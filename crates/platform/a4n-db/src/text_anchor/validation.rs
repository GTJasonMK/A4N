mod anchors;
mod profile;
mod project;
mod rules;
mod snapshot;

pub use anchors::validate_anchor_ref_records;
pub(in crate::text_anchor) use profile::ensure_profiles_belong_to_root;
pub(in crate::text_anchor) use project::ensure_text_root_project;

#[cfg(test)]
mod tests;
