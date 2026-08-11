use super::profiles::AttributeCountsSummary;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotReferenceSummary {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    pub snapshot_hash: String,
    pub reference_count: i32,
    pub attribute_counts: AttributeCountsSummary,
    pub state_summary: Option<String>,
    pub current_location: Option<String>,
    pub current_goal: Option<String>,
    pub emotional_state: Option<String>,
    pub changes_in_chapter: usize,
    pub behaviors_in_chapter: usize,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileChapterStateSummary {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    pub snapshot_hash: String,
    pub attribute_counts: AttributeCountsSummary,
    pub explicit_attributes: Value,
    pub implicit_attributes: Value,
    pub social_attributes: Value,
    pub state_summary: Option<String>,
    pub current_location: Option<String>,
    pub current_goal: Option<String>,
    pub emotional_state: Option<String>,
    pub changes_in_chapter: usize,
    pub behaviors_in_chapter: usize,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListSnapshotsInput {
    pub profile_id: i64,
    pub start_chapter: Option<i32>,
    pub end_chapter: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetProfileStateAtChapterInput {
    pub profile_id: i64,
    pub chapter_number: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotSummary {
    pub content_hash: String,
    pub entity_type: String,
    pub entity_id: i64,
    pub sequence: i64,
    pub data: Value,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetSnapshotInput {
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDeltaSummary {
    pub from_hash: String,
    pub to_hash: String,
    pub delta: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiffProfileChaptersInput {
    pub profile_id: i64,
    pub from_chapter: i32,
    pub to_chapter: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSyncConflictSummary {
    pub profile_id: i64,
    pub has_conflict: bool,
    pub last_synced_chapter: i32,
    pub max_available_chapter: i32,
    pub available_snapshot_chapters: Vec<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckProfileSyncConflictInput {
    pub profile_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResultSummary {
    pub deleted_events: u64,
    pub deleted_snapshots: u64,
    pub deleted_behaviors: u64,
    pub deleted_state_indexes: u64,
    pub deleted_state_rag: usize,
    pub deleted_attribute_event_rag: usize,
    pub deleted_deletion_marks: u64,
    pub rewound_deletion_marks: u64,
    pub last_synced_chapter: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RollbackToChapterInput {
    pub profile_id: i64,
    pub chapter_number: i32,
}
