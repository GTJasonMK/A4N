mod acceptance;
mod dedup;
mod mutations;
mod projections;
mod queries;

pub use a4n_api_contract::import_review::{
    AcceptImportReviewInput, BatchAcceptImportReviewsInput, CheckImportDedupInput, DedupResult,
    ImportProjectReviewsInput, ImportReviewIdInput, ImportReviewSummary,
};
pub use acceptance::{accept_import_review, batch_accept_import_reviews};
pub use dedup::check_import_dedup;
pub use mutations::{delete_import_review, reject_import_review};
pub(crate) use projections::import_review_summary;
pub use queries::list_import_queue;

#[cfg(test)]
mod tests;
