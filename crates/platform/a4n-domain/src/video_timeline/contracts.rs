use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Persisted chapter-video timeline projection. ORM rows are converted to
/// this record at the database boundary before reaching orchestration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoTimelineRecord {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub workflow_mode: String,
    pub revision: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoTimelineItemRecord {
    pub id: i64,
    pub timeline_id: i64,
    pub position: Option<i32>,
    pub item_kind: String,
    pub status: String,
    pub native_source_shot_id: Option<String>,
    pub native_segment_index_in_shot: Option<i32>,
    pub insert_plan_prompt_id: Option<i64>,
    pub workflow_run_id: Option<i64>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoTimelineSnapshotRecord {
    pub timeline: VideoTimelineRecord,
    pub items: Vec<VideoTimelineItemRecord>,
}
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NativeVideoTimelineKey {
    pub source_shot_id: String,
    pub segment_index_in_shot: i32,
}

impl NativeVideoTimelineKey {
    pub fn new(
        source_shot_id: impl Into<String>,
        segment_index_in_shot: i32,
    ) -> Result<Self, String> {
        let source_shot_id = source_shot_id.into().trim().to_string();
        if source_shot_id.is_empty() {
            return Err("source_shot_id 不能为空".to_string());
        }
        if segment_index_in_shot <= 0 {
            return Err("segment_index_in_shot 必须大于 0".to_string());
        }
        Ok(Self {
            source_shot_id,
            segment_index_in_shot,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VideoTimelineGap {
    pub previous_item_id: Option<i64>,
    pub next_item_id: Option<i64>,
}

impl VideoTimelineGap {
    pub fn validate(self) -> Result<Self, TimelineError> {
        if self.previous_item_id.is_some_and(|id| id <= 0)
            || self.next_item_id.is_some_and(|id| id <= 0)
        {
            return Err(TimelineError::InvalidId);
        }
        if self.previous_item_id.is_some() && self.previous_item_id == self.next_item_id {
            return Err(TimelineError::SameNeighbor);
        }
        Ok(self)
    }

    pub fn canonical_key(self) -> Result<String, TimelineError> {
        let gap = self.validate()?;
        Ok(format!(
            "{}:{}",
            gap.previous_item_id.unwrap_or_default(),
            gap.next_item_id.unwrap_or_default(),
        ))
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TimelineError {
    #[error("时间线项 ID 必须为正整数")]
    InvalidId,
    #[error("时间线前后项不能是同一个 ID")]
    SameNeighbor,
    #[error("时间线间隙不属于当前顺序")]
    GapNotFound,
    #[error("时间线中存在重复项或空项")]
    InvalidOrder,
}
