use serde_json::Value;

use super::super::{
    SourceSyncEntityRefRecordInput, SourceSyncMediaRefRecordInput,
    SourceSyncTextAnchorRefRecordInput,
};

#[derive(Debug, Clone)]
pub struct SourceSyncReplacementSpecRecordInput {
    pub generation_spec_id: i64,
    pub prompt_kind: String,
    pub params_json: Value,
    pub text_anchor_refs: Vec<SourceSyncTextAnchorRefRecordInput>,
    pub entity_refs: Vec<SourceSyncEntityRefRecordInput>,
    pub media_refs: Vec<SourceSyncMediaRefRecordInput>,
    pub replacement_segment: SourceSyncReplacementSegmentRecordInput,
}

#[derive(Debug, Clone)]
pub struct SourceSyncReplacementSegmentRecordInput {
    pub title: String,
    pub duration_secs: f32,
    pub order_index: i32,
    pub previous_segment_id: Option<i64>,
    pub next_segment_id: Option<i64>,
}
