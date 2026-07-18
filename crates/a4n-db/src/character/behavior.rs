mod contracts;
mod queries;
mod records;
mod validation;

pub use contracts::InsertBehaviorRecordInput;
pub use queries::{behavior_records_by_profile, implicit_review_behavior_records};
pub use records::{insert_behavior_record, record_behavior_in};
pub use validation::validate_behavior_classification_results;

#[cfg(test)]
mod tests;
