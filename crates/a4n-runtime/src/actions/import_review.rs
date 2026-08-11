mod acceptance;
mod dedup;
mod mutations;
mod projections;
mod queries;

pub use a4n_api_contract::import_review::{
    AcceptImportReviewInput, BatchAcceptImportReviewsInput, CheckImportDedupInput, DedupResult,
    ImportProjectReviewsInput, ImportReviewIdInput, ImportReviewSummary,
};
pub use acceptance::*;
pub use dedup::check_import_dedup;
pub use mutations::*;
pub use projections::import_review_summary;
pub use queries::*;

#[cfg(test)]
mod tests;
