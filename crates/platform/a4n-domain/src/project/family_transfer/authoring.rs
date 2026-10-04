use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyVisualPlanRecord {
    pub id: i64,
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_chapter_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub source_revision_key: String,
    pub source_content_sha256: String,
    pub contract_version: String,
    pub plan_version: i32,
    pub revision: i32,
    pub status: String,
    pub authoring_mode: String,
    pub plan_sha256: String,
    pub plan_json: serde_json::Value,
    pub author_user_id: i64,
    pub confirmed_at: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyShootingScriptRecord {
    pub id: i64,
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_chapter_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub source_revision_key: String,
    pub source_content_sha256: String,
    pub visual_plan_id: Option<i64>,
    pub visual_plan_version: Option<i32>,
    pub visual_plan_sha256: Option<String>,
    pub input_fingerprint_sha256: String,
    pub contract_version: String,
    pub revision: i32,
    pub script_sha256: String,
    pub script_json: serde_json::Value,
    pub workflow_run_id: Option<i64>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyAuthoringDocumentRecord {
    pub id: i64,
    pub project_id: i64,
    pub source_snapshot_id: i64,
    pub source_chapter_id: Option<i64>,
    pub kind: String,
    pub version: i32,
    pub revision: i32,
    pub status: String,
    pub inputs_json: serde_json::Value,
    pub document_json: serde_json::Value,
    pub document_sha256: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyAuthoringMigrationReceiptRecord {
    pub kind: String,
    pub old_id: i64,
    pub new_id: i64,
    pub project_id: i64,
    pub before_json: serde_json::Value,
    pub after_sha256: String,
}
