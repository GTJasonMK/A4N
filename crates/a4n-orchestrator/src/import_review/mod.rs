//! Import review application boundary and validation rules.
//!
//! 数据库行为由 `context/import_review.rs` 的私有 `ImportReviewApplication` 持有。

use a4n_api_contract::import_review::{
    AcceptImportReviewInput, BatchAcceptImportReviewsInput, CheckImportDedupInput, DedupResult,
    ImportProjectReviewsInput, ImportReviewIdInput, ImportReviewSummary,
};
use a4n_domain::import_review::ImportChapterDraft;
use a4n_error::AppError;
use async_trait::async_trait;

use crate::chapter_content::ChapterDerivedCleanupHostPort;

mod rules;

pub(crate) use rules::calculate_import_dedup;

/// Runtime-facing import-review boundary.
///
/// Review validation, authorization, persistence and DTO projection stay in
/// the import-review owner. Runtime only parses files and supplies cleanup
/// side effects for accepted chapters.
#[async_trait]
pub trait ImportReviewApplicationPort: Send + Sync {
    async fn create_from_parsed(
        &self,
        user_id: i64,
        project_id: i64,
        filename: &str,
        chapters: &[ImportChapterDraft],
    ) -> Result<Vec<ImportReviewSummary>, AppError>;

    async fn list(
        &self,
        user_id: i64,
        input: ImportProjectReviewsInput,
    ) -> Result<Vec<ImportReviewSummary>, AppError>;

    async fn accept(
        &self,
        user_id: i64,
        input: AcceptImportReviewInput,
        host_port: &dyn ChapterDerivedCleanupHostPort,
    ) -> Result<ImportReviewSummary, AppError>;

    async fn batch_accept(
        &self,
        user_id: i64,
        input: BatchAcceptImportReviewsInput,
        host_port: &dyn ChapterDerivedCleanupHostPort,
    ) -> Result<Vec<ImportReviewSummary>, AppError>;

    async fn delete(&self, user_id: i64, input: ImportReviewIdInput) -> Result<(), AppError>;

    async fn reject(
        &self,
        user_id: i64,
        input: ImportReviewIdInput,
    ) -> Result<ImportReviewSummary, AppError>;

    async fn dedup(
        &self,
        user_id: i64,
        input: CheckImportDedupInput,
    ) -> Result<DedupResult, AppError>;
}
