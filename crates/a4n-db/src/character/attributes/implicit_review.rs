mod execution;
mod validation;

pub use execution::apply_implicit_review_decision_record;
pub(in crate::character) use execution::apply_implicit_review_decision_record_with_profile_from_source_in;
#[cfg(test)]
pub(in crate::character::attributes) use validation::require_positive_chapter_number;

#[cfg(test)]
mod tests;
