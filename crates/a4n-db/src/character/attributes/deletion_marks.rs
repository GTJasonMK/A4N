mod queries;
mod requests;
mod rewind;
mod rules;
mod updates;
mod upsert;

pub use queries::deletion_marks_by_profile;
pub(in crate::character::attributes) use queries::pending_deletion_mark_for_attribute;
pub(in crate::character) use requests::request_attribute_deletion_from_source_in;
#[cfg(test)]
pub(in crate::character::attributes) use requests::request_attribute_deletion_in;
pub use requests::{request_attribute_deletion, reset_deletion_marks_in};
pub(in crate::character) use rewind::rewind_ai_deletion_marks_after;
pub use rewind::rewind_deletion_marks_after;
#[cfg(test)]
use rules::{next_deletion_mark_count, retained_deletion_mark_count};
pub(in crate::character::attributes) use updates::{
    mark_deletion_executed, reset_pending_deletion_marks,
};

#[cfg(test)]
mod tests;
