use sea_orm::DbErr;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedChapterSourceKind {
    Live,
    Snapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResolvedChapterSource {
    pub source_kind: ResolvedChapterSourceKind,
    pub requested_project_id: i64,
    pub root_project_id: i64,
    pub source_chapter_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub analysis_data_json: Option<Value>,
    pub content: String,
    pub content_sha256: String,
    pub source_revision_key: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ResolveChapterSourceError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    InvalidState(String),
    #[error("{0}")]
    NotFound(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}
