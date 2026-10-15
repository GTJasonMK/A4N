use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parsed chapter payload exchanged between format adapters and the import
/// application. It contains no parser, file, database, or Host types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportChapterDraft {
    pub number: i32,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportReviewCreateInput {
    pub project_id: i64,
    pub filename: String,
    pub analysis_result: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportReviewRecord {
    pub id: i64,
    pub project_id: i64,
    pub filename: String,
    pub analysis_result: Value,
    pub review_status: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportReviewListRecord {
    pub id: i64,
    pub project_id: i64,
    pub filename: String,
    pub review_status: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ImportReviewStatus {
    Pending,
    Accepted,
    Rejected,
    Modified,
}

impl ImportReviewStatus {
    pub const ALL: [Self; 4] = [
        Self::Pending,
        Self::Accepted,
        Self::Rejected,
        Self::Modified,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Modified => "modified",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }
}
