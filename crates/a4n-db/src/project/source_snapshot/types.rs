use a4n_core::models::chapter::chapter;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DerivedChapterScope {
    pub start_chapter: i32,
    pub end_chapter: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateDerivedProjectParams {
    pub root_project_id: i64,
    pub derived_kind: String,
    pub title: String,
    pub description: String,
    pub chapter_scope: DerivedChapterScope,
    pub creation_options_json: Value,
}

pub(super) struct SourceSnapshotBuildData {
    pub(super) chapters: Vec<chapter::Model>,
    pub(super) chapter_hashes_json: Vec<Value>,
    pub(super) profile_hashes_json: Value,
    pub(super) latest_blueprint_version_id: Option<i64>,
    pub(super) chapter_scope_json: Value,
    pub(super) source_version_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSnapshotChapterSummary {
    pub id: i64,
    pub snapshot_id: i64,
    pub root_project_id: i64,
    pub source_chapter_id: i64,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub content: String,
    pub analysis_data_json: Option<Value>,
    pub content_hash: String,
    pub analysis_hash: Option<String>,
}
