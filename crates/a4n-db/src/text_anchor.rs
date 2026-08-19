mod current;
mod evidence;
mod insertion;
mod types;
mod validation;

pub use current::{
    ensure_chapter_content_anchor_records, read_current_chapter_content_anchor_records,
    rebuild_chapter_content_anchor_records,
};
pub use evidence::{
    delete_chapter_content_profile_evidence_records, earliest_current_profile_evidence_chapter,
    merge_profile_evidence_records,
};
pub(crate) use insertion::insert_content_anchor_records;
pub(crate) use types::TextAnchorContentInsertInput;
use types::TextAnchorDbResult;
pub use types::{
    CurrentChapterContentAnchorRows, TextAnchorDbError, TextAnchorEvidenceInput,
    TextAnchorProfileKind,
};
pub use validation::validate_anchor_ref_records;

pub const SOURCE_KIND_CHAPTER_CONTENT: &str = "chapter_content";
pub const EVIDENCE_ROLE_FIRST_SEEN: &str = "first_seen";
pub const EVIDENCE_ROLE_APPEARANCE: &str = "appearance";
pub const EVIDENCE_ROLE_STATE_CHANGE: &str = "state_change";
pub const EVIDENCE_ROLE_LOCATION: &str = "location";
pub const EVIDENCE_ROLE_RELATIONSHIP: &str = "relationship";
pub const EVIDENCE_ROLE_BEHAVIOR: &str = "behavior";
pub(super) const PROJECT_KIND_TEXT: &str = "text";

#[cfg(test)]
mod tests;
