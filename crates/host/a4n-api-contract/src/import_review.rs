use a4n_domain::import_review::ImportReviewStatus;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ImportReviewSummary {
    pub id: i64,
    pub project_id: i64,
    pub filename: String,
    pub analysis_result: Value,
    pub review_status: ImportReviewStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportProjectReviewsInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportReviewIdInput {
    pub review_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AcceptImportReviewInput {
    pub review_id: i64,
    pub analysis_result: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BatchAcceptImportReviewsInput {
    pub review_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct BatchAcceptImportReviewsFailure {
    pub review_id: i64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct BatchAcceptImportReviewsResult {
    pub accepted_reviews: Vec<ImportReviewSummary>,
    pub failure: Option<BatchAcceptImportReviewsFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct DedupResult {
    pub content_hash: String,
    pub is_duplicate: bool,
    pub closest_distance: Option<u32>,
    pub matched_chapter_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CheckImportDedupInput {
    pub content: String,
    pub project_id: i64,
}

/// Host-facing read boundary for import reviews.
///
/// Review creation and state transitions remain with the import-review owner.
#[async_trait::async_trait]
pub trait ImportReviewReadApplicationPort: Send + Sync {
    async fn list(
        &self,
        user_id: i64,
        input: ImportProjectReviewsInput,
    ) -> Result<Vec<ImportReviewSummary>, a4n_error::AppError>;

    async fn dedup(
        &self,
        user_id: i64,
        input: CheckImportDedupInput,
    ) -> Result<DedupResult, a4n_error::AppError>;
}

/// Host-facing import-review acceptance boundary.
///
/// Chapter-derived cleanup is supplied by the Host adapter.
#[async_trait::async_trait]
pub trait ImportReviewAcceptanceApplicationPort: Send + Sync {
    async fn accept(
        &self,
        user_id: i64,
        input: AcceptImportReviewInput,
        host_port: &dyn crate::chapter::ChapterDerivedCleanupHostPort,
    ) -> Result<ImportReviewSummary, a4n_error::AppError>;

    async fn batch_accept(
        &self,
        user_id: i64,
        input: BatchAcceptImportReviewsInput,
        host_port: &dyn crate::chapter::ChapterDerivedCleanupHostPort,
    ) -> Result<BatchAcceptImportReviewsResult, a4n_error::AppError>;
}

/// Host-facing import-review state mutation boundary.
#[async_trait::async_trait]
pub trait ImportReviewMutationApplicationPort: Send + Sync {
    async fn delete(
        &self,
        user_id: i64,
        input: ImportReviewIdInput,
    ) -> Result<(), a4n_error::AppError>;

    async fn reject(
        &self,
        user_id: i64,
        input: ImportReviewIdInput,
    ) -> Result<ImportReviewSummary, a4n_error::AppError>;
}
