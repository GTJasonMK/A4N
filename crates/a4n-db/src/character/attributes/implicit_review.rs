mod execution;
mod validation;

pub(in crate::character) use execution::apply_implicit_review_decision_record_from_source_in;
pub use execution::{
    apply_implicit_review_decision_record, apply_implicit_review_decision_record_in,
};
#[cfg(test)]
pub(in crate::character::attributes) use validation::required_positive_chapter_number;

#[cfg(test)]
mod tests;
