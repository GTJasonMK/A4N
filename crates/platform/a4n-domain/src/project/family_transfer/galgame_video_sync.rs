use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyGalgameScriptRecord {
    pub id: i64,
    pub project_id: i64,
    pub root_project_id: i64,
    /// Source of the full-script generation; each scene owns its current source.
    pub source_snapshot_id: i64,
    pub branch_plan_id: Option<i64>,
    pub title: String,
    pub style_config_json: serde_json::Value,
    pub status: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyGalgameSceneRecord {
    pub id: i64,
    pub script_id: i64,
    pub source_snapshot_chapter_id: i64,
    pub adaptation_unit_id: Option<String>,
    pub adaptation_plan_id: Option<i64>,
    pub scene_index: i32,
    pub title: String,
    pub location: Option<String>,
    pub time_of_day: Option<String>,
    pub participants_json: serde_json::Value,
    pub background_note: Option<String>,
    pub entry_kind: String,
    pub next_scene_id: Option<i64>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyGalgameLineRecord {
    pub id: i64,
    pub scene_id: i64,
    pub line_index: i32,
    pub line_type: String,
    pub speaker_profile_id: Option<i64>,
    pub text: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
    pub position: Option<String>,
    pub presentation_json: serde_json::Value,
    pub text_anchor_id: Option<i64>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyGalgameChoiceRecord {
    pub id: i64,
    pub line_id: i64,
    pub option_index: i32,
    pub option_text: String,
    pub target_scene_id: Option<i64>,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyVideoSegmentRecord {
    pub id: i64,
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub segment_type: String,
    pub order_index: i32,
    pub title: String,
    pub duration_secs: f32,
    pub source_generation_spec_id: Option<i64>,
    pub result_media_asset_id: Option<i64>,
    pub previous_segment_id: Option<i64>,
    pub next_segment_id: Option<i64>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyAssemblyTimelineRecord {
    pub id: i64,
    pub project_id: i64,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub version: i32,
    pub status: String,
    pub preview_media_asset_id: Option<i64>,
    pub preview_cache_key: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyAssemblyClipRecord {
    pub id: i64,
    pub timeline_id: i64,
    pub media_asset_id: i64,
    pub order_index: i32,
    pub trim_start_secs: f32,
    pub trim_end_secs: f32,
    pub volume: f32,
    pub transition_kind: String,
    pub source_segment_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilySourceSyncRunRecord {
    pub id: i64,
    pub derived_project_id: i64,
    pub root_project_id: i64,
    pub from_snapshot_id: i64,
    pub to_snapshot_id: i64,
    pub mode: String,
    pub status: String,
    pub summary_json: serde_json::Value,
    pub error_message: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilySourceSyncConflictRecord {
    pub id: i64,
    pub sync_run_id: i64,
    pub derived_project_id: i64,
    pub root_project_id: i64,
    pub from_snapshot_id: i64,
    pub to_snapshot_id: i64,
    pub conflict_scope: String,
    pub target_kind: String,
    pub target_id: i64,
    pub source_change_kind: String,
    pub local_state_kind: String,
    pub severity: String,
    pub status: String,
    pub resolution_strategy: Option<String>,
    pub source_summary_json: serde_json::Value,
    pub local_summary_json: serde_json::Value,
    pub affected_refs_json: serde_json::Value,
    pub decision_json: Option<serde_json::Value>,
    pub created_at: chrono::NaiveDateTime,
    pub resolved_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilySourceSyncDecisionRecord {
    pub id: i64,
    pub conflict_id: i64,
    pub decided_by: String,
    pub strategy: String,
    pub decision_payload_json: serde_json::Value,
    pub created_at: chrono::NaiveDateTime,
}
