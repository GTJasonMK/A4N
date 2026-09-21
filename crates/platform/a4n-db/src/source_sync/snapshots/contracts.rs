use a4n_domain::source_sync::SourceSyncChangeKind;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct SourceSnapshotDiffRecord {
    pub chapter_changes: Vec<SourceSnapshotChapterChangeRecord>,
    pub profile_changes: Vec<SourceSnapshotProfileChangeRecord>,
}

#[derive(Debug, Clone)]
pub struct SourceSnapshotChapterChangeRecord {
    pub source_chapter_id: i64,
    pub source_snapshot_chapter_id: Option<i64>,
    pub change_kind: SourceSyncChangeKind,
    pub source_summary_json: Value,
}

#[derive(Debug, Clone)]
pub struct SourceSnapshotProfileChangeRecord {
    pub profile_kind: String,
    pub source_profile_id: i64,
    pub change_kind: SourceSyncChangeKind,
    pub source_summary_json: Value,
}
