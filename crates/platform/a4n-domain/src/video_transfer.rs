use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoSegmentRecord {
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
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssemblyTimelineRecord {
    pub id: i64,
    pub project_id: i64,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub version: i32,
    pub status: String,
    pub preview_media_asset_id: Option<i64>,
    pub preview_cache_key: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssemblyClipRecord {
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
pub struct VideoStudioMediaAssetRecord {
    pub id: i64,
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub media_kind: String,
    pub origin: String,
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_secs: Option<f32>,
    pub fps: Option<f32>,
    pub file_size: Option<i64>,
    pub checksum: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
