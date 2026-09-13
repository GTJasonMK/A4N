//! Import review application boundary and validation rules.
//!
//! 数据库行为由 `context/import_review.rs` 的私有 `ImportReviewApplication` 持有。

use a4n_api_contract::import_review::ImportReviewSummary;
use a4n_domain::import_review::ImportChapterDraft;
use a4n_error::AppError;
use async_trait::async_trait;

mod rules;

pub(crate) use rules::calculate_import_dedup;

/// Runtime-facing import-review boundary.
///
/// Review validation, authorization, persistence and DTO projection stay in
/// the import-review owner. Runtime only parses files and supplies cleanup
/// side effects for accepted chapters.
#[async_trait]
pub trait ImportReviewIngestionApplicationPort: Send + Sync {
    async fn create_from_parsed(
        &self,
        user_id: i64,
        project_id: i64,
        filename: &str,
        chapters: &[ImportChapterDraft],
    ) -> Result<Vec<ImportReviewSummary>, AppError>;
}
