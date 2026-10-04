use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilySourceSnapshotRecord {
    pub id: i64,
    pub root_project_id: i64,
    pub source_version_hash: String,
    pub blueprint_version_id: Option<i64>,
    pub chapter_scope_json: serde_json::Value,
    pub chapter_hashes_json: serde_json::Value,
    pub profile_hashes_json: serde_json::Value,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilySourceSnapshotChapterRecord {
    pub id: i64,
    pub snapshot_id: i64,
    pub root_project_id: i64,
    pub source_chapter_id: i64,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub content: String,
    pub summary: Option<String>,
    pub analysis_data_json: Option<serde_json::Value>,
    pub content_hash: String,
    pub analysis_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilySourceSnapshotOutlineRecord {
    pub id: i64,
    pub snapshot_id: i64,
    pub root_project_id: i64,
    pub source_outline_id: i64,
    pub outline_kind: String,
    pub payload_json: serde_json::Value,
    pub payload_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilySourceSnapshotProfileRecord {
    pub id: i64,
    pub snapshot_id: i64,
    pub root_project_id: i64,
    pub profile_kind: String,
    pub source_profile_id: i64,
    pub payload_json: serde_json::Value,
    pub payload_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyTextAnchorRecord {
    pub id: i64,
    pub project_id: i64,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub source_kind: String,
    pub source_id: i64,
    pub paragraph_index: i32,
    pub start_offset: i32,
    pub end_offset: i32,
    pub quote: String,
    pub quote_hash: String,
    pub source_version_hash: String,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyProfileEvidenceRecord {
    pub id: i64,
    pub profile_kind: String,
    pub profile_id: i64,
    pub text_anchor_id: i64,
    pub evidence_role: String,
    pub created_at: chrono::NaiveDateTime,
}
