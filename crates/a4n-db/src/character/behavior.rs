mod contracts;
mod queries;
mod records;
mod validation;

pub use contracts::InsertBehaviorRecordInput;
pub use queries::{behavior_records_by_profile, implicit_review_behavior_records};
pub(in crate::character) use records::record_behavior_in;
#[cfg(test)]
use validation::validate_behavior_classification_results;

#[cfg(test)]
mod tests;
