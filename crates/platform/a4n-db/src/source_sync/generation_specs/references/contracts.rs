use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
pub struct SourceSyncAnchorMappingRecord {
    pub old_text_anchor_id: i64,
    pub new_text_anchor_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSyncTextAnchorRefRecordInput {
    pub text_anchor_id: i64,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSyncEntityRefRecordInput {
    pub entity_kind: String,
    pub entity_id: i64,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSyncMediaRefRecordInput {
    pub media_asset_id: i64,
    pub role: String,
}
